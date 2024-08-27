mod color;
mod config;
mod key;
mod menu;
mod text;

use std::collections::HashSet;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use clap::Parser;
use pangocairo::cairo;

use wayrs_client::object::ObjectId;
use wayrs_client::protocol::*;
use wayrs_client::proxy::Proxy;
use wayrs_client::{global::*, EventCtx};
use wayrs_client::{Connection, IoMode};
use wayrs_protocols::wlr_layer_shell_unstable_v1::*;
use wayrs_utils::keyboard::{Keyboard, KeyboardEvent, KeyboardHandler};
use wayrs_utils::seats::{SeatHandler, Seats};
use wayrs_utils::shm_alloc::{BufferSpec, ShmAlloc};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// The name of the config file to use.
    ///
    /// By default, $XDG_CONFIG_HOME/wlr-which-key/config.yaml or
    /// ~/.config/wlr-which-key/config.yaml is used.
    ///
    /// For example, to use ~/.config/wlr-which-key/print-srceen.yaml, set this to
    /// "print-srceen". An absolute path can be used too, extension is optional.
    config: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config::Config::new(args.config.as_deref().unwrap_or("config"))?;
    let menu = menu::Menu::new(&config)?;

    let (mut conn, globals) = Connection::connect_and_collect_globals()?;
    conn.add_registry_cb(wl_registry_cb);

    let wl_compositor: WlCompositor = globals.bind(&mut conn, 4..=6)?;
    let wlr_layer_shell: ZwlrLayerShellV1 = globals.bind(&mut conn, 2)?;

    let seats = Seats::bind(&mut conn, &globals);
    let shm_alloc = ShmAlloc::bind(&mut conn, &globals)?;

    let width = (menu.width() + (config.padding() + config.border_width) * 2.0) as u32;
    let height = (menu.height() + (config.padding() + config.border_width) * 2.0) as u32;

    let wl_surface = wl_compositor.create_surface_with_cb(&mut conn, wl_surface_cb);

    let layer_surface = wlr_layer_shell.get_layer_surface_with_cb(
        &mut conn,
        wl_surface,
        None,
        zwlr_layer_shell_v1::Layer::Overlay,
        wayrs_client::cstr!("wlr_which_key").into(),
        layer_surface_cb,
    );
    layer_surface.set_anchor(&mut conn, config.anchor.into());
    layer_surface.set_size(&mut conn, width, height);
    layer_surface.set_margin(
        &mut conn,
        config.margin_top,
        config.margin_right,
        config.margin_bottom,
        config.margin_left,
    );
    layer_surface.set_keyboard_interactivity(
        &mut conn,
        zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
    );
    wl_surface.commit(&mut conn);

    let mut state = State {
        shm_alloc,
        seats,
        keyboards: Vec::new(),
        outputs: Vec::new(),

        wl_surface,
        layer_surface,
        visible_on_outputs: HashSet::new(),
        surface_scale: 1,
        exit: false,
        configured: false,
        width,
        height,
        throttle_cb: None,
        throttled: false,

        menu,
        config,
    };

    globals
        .iter()
        .filter(|g| g.is::<WlOutput>())
        .for_each(|g| state.bind_output(&mut conn, g));

    while !state.exit {
        conn.flush(IoMode::Blocking)?;
        conn.recv_events(IoMode::Blocking)?;
        conn.dispatch_events(&mut state);
    }

    Ok(())
}

struct State {
    shm_alloc: ShmAlloc,
    seats: Seats,
    keyboards: Vec<Keyboard>,
    outputs: Vec<Output>,

    wl_surface: WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    visible_on_outputs: HashSet<ObjectId>,
    surface_scale: u32,
    exit: bool,
    configured: bool,
    width: u32,
    height: u32,
    throttle_cb: Option<WlCallback>,
    throttled: bool,

    menu: menu::Menu,
    config: config::Config,
}

struct Output {
    wl: WlOutput,
    reg_name: u32,
    scale: u32,
}

impl State {
    fn draw(&mut self, conn: &mut Connection<Self>) {
        if !self.configured {
            return;
        }

        if self.throttle_cb.is_some() {
            self.throttled = true;
            return;
        }

        self.throttle_cb = Some(self.wl_surface.frame_with_cb(conn, |ctx| {
            assert_eq!(ctx.state.throttle_cb, Some(ctx.proxy));
            ctx.state.throttle_cb = None;
            if ctx.state.throttled {
                ctx.state.throttled = false;
                ctx.state.draw(ctx.conn);
            }
        }));

        let scale = if self.wl_surface.version() >= 6 {
            self.surface_scale
        } else {
            self.outputs
                .iter()
                .filter(|o| self.visible_on_outputs.contains(&o.wl.id()))
                .map(|o| o.scale)
                .max()
                .unwrap_or(1)
        };

        let width_f = self.width as f64;
        let height_f = self.height as f64;

        let (buffer, canvas) = self
            .shm_alloc
            .alloc_buffer(
                conn,
                BufferSpec {
                    width: self.width * scale,
                    height: self.height * scale,
                    stride: self.width * 4 * scale,
                    format: wl_shm::Format::Argb8888,
                },
            )
            .expect("could not allocate frame shm buffer");

        let cairo_surf = unsafe {
            cairo::ImageSurface::create_for_data_unsafe(
                canvas.as_mut_ptr(),
                cairo::Format::ARgb32,
                (self.width * scale) as i32,
                (self.height * scale) as i32,
                (self.width * 4 * scale) as i32,
            )
            .expect("cairo surface")
        };

        let cairo_ctx = cairo::Context::new(&cairo_surf).expect("cairo context");
        cairo_ctx.scale(scale as f64, scale as f64);
        self.wl_surface.set_buffer_scale(conn, scale as i32);

        // background with rounded corners
        cairo_ctx.save().unwrap();
        cairo_ctx.set_operator(cairo::Operator::Source);
        color::Color::TRANSPARENT.apply(&cairo_ctx);
        cairo_ctx.paint().unwrap();
        cairo_ctx.restore().unwrap();

        cairo_ctx.new_sub_path();
        let half_border = self.config.border_width * 0.5;
        let r = self.config.corner_r;
        cairo_ctx.arc(r + half_border, r + half_border, r, PI, 3.0 * FRAC_PI_2);
        cairo_ctx.arc(
            width_f - r - half_border,
            r + half_border,
            r,
            3.0 * FRAC_PI_2,
            TAU,
        );
        cairo_ctx.arc(
            width_f - r - half_border,
            height_f - r - half_border,
            r,
            0.0,
            FRAC_PI_2,
        );
        cairo_ctx.arc(
            r + half_border,
            height_f - r - half_border,
            r,
            FRAC_PI_2,
            PI,
        );
        cairo_ctx.close_path();
        self.config.background.apply(&cairo_ctx);
        cairo_ctx.fill_preserve().unwrap();
        self.config.border.apply(&cairo_ctx);
        cairo_ctx.set_line_width(self.config.border_width);
        cairo_ctx.stroke().unwrap();

        // draw our menu
        self.menu
            .render(
                &self.config,
                &cairo_ctx,
                self.config.padding() + self.config.border_width,
                self.config.padding() + self.config.border_width,
            )
            .unwrap();

        // Damage the entire window
        self.wl_surface.damage_buffer(
            conn,
            0,
            0,
            (self.width * scale) as i32,
            (self.height * scale) as i32,
        );

        // Attach and commit to present.
        self.wl_surface
            .attach(conn, Some(buffer.into_wl_buffer()), 0, 0);
        self.wl_surface.commit(conn);
    }

    fn bind_output(&mut self, conn: &mut Connection<Self>, global: &Global) {
        let wl: WlOutput = global.bind_with_cb(conn, 1..=4, wl_output_cb).unwrap();
        self.outputs.push(Output {
            wl,
            reg_name: global.name,
            scale: 1,
        });
    }
}

impl SeatHandler for State {
    fn get_seats(&mut self) -> &mut Seats {
        &mut self.seats
    }

    fn keyboard_added(&mut self, conn: &mut Connection<Self>, seat: WlSeat) {
        self.keyboards.push(Keyboard::new(conn, seat));
    }

    fn keyboard_removed(&mut self, conn: &mut Connection<Self>, seat: WlSeat) {
        let i = self
            .keyboards
            .iter()
            .position(|k| k.seat() == seat)
            .unwrap();
        let keyboard = self.keyboards.swap_remove(i);
        keyboard.destroy(conn);
    }
}

impl KeyboardHandler for State {
    fn get_keyboard(&mut self, wl_keyboard: WlKeyboard) -> &mut Keyboard {
        self.keyboards
            .iter_mut()
            .find(|k| k.wl_keyboard() == wl_keyboard)
            .unwrap()
    }

    fn key_presed(&mut self, conn: &mut Connection<Self>, event: KeyboardEvent) {
        if let Some(action) = self.menu.get_action(&event.xkb_state, event.keysym) {
            match action {
                menu::Action::Quit => {
                    self.exit = true;
                    conn.break_dispatch_loop();
                }
                menu::Action::Exec(cmd) => {
                    let mut proc = Command::new("sh");
                    proc.args(["-c", &cmd]);
                    proc.stdin(Stdio::null());
                    proc.stdout(Stdio::null());
                    // Safety: libc::daemon() is async-signal-safe
                    unsafe {
                        proc.pre_exec(|| match libc::daemon(1, 0) {
                            -1 => Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Failed to detach new process",
                            )),
                            _ => Ok(()),
                        });
                    }
                    proc.spawn().unwrap().wait().unwrap();
                    self.exit = true;
                }
                menu::Action::Submenu(page) => {
                    self.menu.set_page(page);

                    self.width = (self.menu.width()
                        + (self.config.padding() + self.config.border_width) * 2.0)
                        as u32;
                    self.height = (self.menu.height()
                        + (self.config.padding() + self.config.border_width) * 2.0)
                        as u32;

                    self.layer_surface.set_size(conn, self.width, self.height);
                    self.wl_surface.commit(conn);
                }
            }
        }
    }

    fn key_released(&mut self, _: &mut Connection<Self>, _: KeyboardEvent) {}
}

fn wl_registry_cb(conn: &mut Connection<State>, state: &mut State, event: &wl_registry::Event) {
    match event {
        wl_registry::Event::Global(g) if g.is::<WlOutput>() => state.bind_output(conn, g),
        wl_registry::Event::GlobalRemove(name) => {
            if let Some(output_i) = state.outputs.iter().position(|o| o.reg_name == *name) {
                let output = state.outputs.swap_remove(output_i);
                state.visible_on_outputs.remove(&output.wl.id());
                if output.wl.version() >= 3 {
                    output.wl.release(conn);
                }
            }
        }
        _ => (),
    }
}

fn wl_output_cb(ctx: EventCtx<State, WlOutput>) {
    if let wl_output::Event::Scale(scale) = ctx.event {
        let output = ctx
            .state
            .outputs
            .iter_mut()
            .find(|o| o.wl == ctx.proxy)
            .unwrap();
        let scale: u32 = scale.try_into().unwrap();
        if output.scale != scale {
            output.scale = scale;
            ctx.state.draw(ctx.conn);
        }
    }
}

fn wl_surface_cb(ctx: EventCtx<State, WlSurface>) {
    assert_eq!(ctx.proxy, ctx.state.wl_surface);
    match ctx.event {
        wl_surface::Event::Enter(output) => {
            ctx.state.visible_on_outputs.insert(output);
            ctx.state.draw(ctx.conn);
        }
        wl_surface::Event::Leave(output) => {
            ctx.state.visible_on_outputs.remove(&output);
        }
        wl_surface::Event::PreferredBufferScale(scale) => {
            assert!(scale >= 1);
            let scale = scale as u32;
            if ctx.state.surface_scale != scale {
                ctx.state.surface_scale = scale;
                ctx.state.draw(ctx.conn);
            }
        }
        _ => (),
    }
}

fn layer_surface_cb(ctx: EventCtx<State, ZwlrLayerSurfaceV1>) {
    assert_eq!(ctx.proxy, ctx.state.layer_surface);
    match ctx.event {
        zwlr_layer_surface_v1::Event::Configure(args) => {
            if args.width != 0 {
                ctx.state.width = args.width;
            }
            if args.height != 0 {
                ctx.state.height = args.height;
            }
            ctx.state.configured = true;
            ctx.proxy.ack_configure(ctx.conn, args.serial);
            ctx.state.draw(ctx.conn);
        }
        zwlr_layer_surface_v1::Event::Closed => {
            ctx.state.exit = true;
            ctx.conn.break_dispatch_loop();
        }
        _ => (),
    }
}
