mod color;
mod config;
mod key;
mod menu;
mod text;

use std::collections::{HashMap, HashSet};
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::bail;
use clap::Parser;
use pangocairo::cairo;

use wayrs_client::object::ObjectId;
use wayrs_client::protocol::*;
use wayrs_client::proxy::Proxy;
use wayrs_client::{Connection, IoMode};
use wayrs_client::{EventCtx, global::*};
use wayrs_protocols::keyboard_shortcuts_inhibit_unstable_v1::*;
use wayrs_protocols::wlr_layer_shell_unstable_v1::*;
use wayrs_utils::keyboard::{Keyboard, KeyboardEvent, KeyboardHandler, xkb};
use wayrs_utils::seats::{SeatHandler, Seats};
use wayrs_utils::shm_alloc::{BufferSpec, ShmAlloc};
use wayrs_utils::timer::Timer;

use crate::key::ModifierState;

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

    /// Initial key sequence to navigate to a specific submenu on startup.
    ///
    /// Provide a sequence of keys separated by spaces to navigate directly to a submenu.
    /// For example, "p s" would navigate to the submenu at key 'p', then 's'.
    /// The application will show an error and exit if the key sequence is invalid.
    #[arg(long, short = 'k')]
    initial_keys: Option<String>,
}

static DEBUG_LAYOUT: LazyLock<bool> =
    LazyLock::new(|| std::env::var("WLR_WHICH_KEY_LAYOUT_DEBUG").as_deref() == Ok("1"));

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config::Config::new(args.config.as_deref().unwrap_or("config"))?;
    let mut menu = menu::Menu::new(&config)?;

    if let Some(initial_keys) = &args.initial_keys
        && let Some(initial_action) = menu.navigate_to_key_sequence(initial_keys)?
    {
        match initial_action {
            menu::Action::Submenu(_) => unreachable!(),
            menu::Action::Quit => return Ok(()),
            menu::Action::Exec { cmd, keep_open } => {
                if keep_open {
                    bail!("Initial key sequence cannot trigger an action with keep_open=true");
                }
                exec(&cmd);
                return Ok(());
            }
        }
    }

    let mut conn = Connection::connect()?;
    conn.blocking_roundtrip()?;
    conn.add_registry_cb(wl_registry_cb);

    let wl_compositor: WlCompositor = conn.bind_singleton(4..=6)?;
    let wlr_layer_shell: ZwlrLayerShellV1 = conn.bind_singleton(2)?;
    let keyboard_shortcuts_inhibit_manager = match config.inhibit_compositor_keyboard_shortcuts {
        true => Some(conn.bind_singleton(1)?),
        false => None,
    };

    let seats = Seats::new(&mut conn);
    let shm_alloc = ShmAlloc::bind(&mut conn)?;

    let width = menu.width(&config) as u32;
    let height = menu.height(&config) as u32;

    let wl_surface = wl_compositor.create_surface_with_cb(&mut conn, wl_surface_cb);

    let layer_surface = wlr_layer_shell.get_layer_surface_with_cb(
        &mut conn,
        wl_surface,
        None,
        zwlr_layer_shell_v1::Layer::Overlay,
        config.namespace.0.to_owned(),
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
        kbd_repeat: None,
        outputs: Vec::new(),
        keyboard_shortcuts_inhibit_manager,
        keyboard_shortcuts_inhibitors: HashMap::new(),

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

    while !state.exit {
        conn.flush(IoMode::Blocking)?;

        poll(
            conn.as_raw_fd(),
            state.kbd_repeat.as_ref().map(|x| x.0.sleep()),
        )?;

        if let Some((timer, action)) = &mut state.kbd_repeat
            && timer.tick()
        {
            let action = action.clone();
            state.handle_action(&mut conn, action);
        }

        match conn.recv_events(IoMode::NonBlocking) {
            Ok(()) => conn.dispatch_events(&mut state),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

struct State {
    shm_alloc: ShmAlloc,
    seats: Seats,
    keyboards: Vec<Keyboard>,
    kbd_repeat: Option<(Timer, menu::Action)>,
    outputs: Vec<Output>,
    keyboard_shortcuts_inhibit_manager: Option<ZwpKeyboardShortcutsInhibitManagerV1>,
    keyboard_shortcuts_inhibitors: HashMap<WlSeat, ZwpKeyboardShortcutsInhibitorV1>,

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
        self.menu.render(&self.config, &cairo_ctx).unwrap();

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

    fn handle_action(&mut self, conn: &mut Connection<Self>, action: menu::Action) {
        match action {
            menu::Action::Quit => {
                self.exit = true;
                conn.break_dispatch_loop();
            }
            menu::Action::Exec { cmd, keep_open } => {
                exec(&cmd);
                if !keep_open {
                    self.exit = true;
                    conn.break_dispatch_loop();
                }
            }
            menu::Action::Submenu(page) => {
                self.menu.set_page(page);
                self.width = self.menu.width(&self.config) as u32;
                self.height = self.menu.height(&self.config) as u32;
                self.layer_surface.set_size(conn, self.width, self.height);
                self.wl_surface.commit(conn);
            }
        };
        self.draw(conn);
    }
}

impl SeatHandler for State {
    fn get_seats(&mut self) -> &mut Seats {
        &mut self.seats
    }

    fn seat_added(&mut self, conn: &mut Connection<Self>, seat: WlSeat) {
        if let Some(inhibit_manager) = self.keyboard_shortcuts_inhibit_manager {
            self.keyboard_shortcuts_inhibitors.insert(
                seat,
                inhibit_manager.inhibit_shortcuts(conn, self.wl_surface, seat),
            );
        }
    }

    fn seat_removed(&mut self, conn: &mut Connection<Self>, seat: WlSeat) {
        if let Some(inhibitor) = self.keyboard_shortcuts_inhibitors.remove(&seat) {
            inhibitor.destroy(conn);
        }
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
        self.kbd_repeat = None;
        let modifiers = ModifierState::from_xkb_state(&event.xkb_state);
        let action = if let Some(action) = self.menu.get_action(modifiers, event.keysym) {
            Some(action)
        } else if self.config.auto_kbd_layout {
            let mask = XkbMaskState::new(&event.xkb_state);
            let mut action = None;
            // Try each layout
            for layout in 0..event.xkb_state.get_keymap().num_layouts() {
                mask.with_locked_layout(layout).apply(&event.xkb_state);
                if let Some(a) = self
                    .menu
                    .get_action(modifiers, event.xkb_state.key_get_one_sym(event.keycode))
                {
                    action = Some(a);
                    break;
                }
            }
            mask.apply(&event.xkb_state); // Restore the state
            action
        } else {
            None
        };
        if let Some(action) = action {
            if let Some(repeat) = event.repeat_info {
                self.kbd_repeat = Some((Timer::new(repeat.delay, repeat.interval), action.clone()));
            }
            self.handle_action(conn, action);
        }
    }

    fn key_released(&mut self, _: &mut Connection<Self>, _: KeyboardEvent) {
        self.kbd_repeat = None;
    }
}

fn wl_registry_cb(conn: &mut Connection<State>, state: &mut State, event: &wl_registry::Event) {
    match event {
        wl_registry::Event::Global(g) if g.is::<WlOutput>() => {
            state.outputs.push(Output {
                wl: g.bind_with_cb(conn, 1..=4, wl_output_cb).unwrap(),
                reg_name: g.name,
                scale: 1,
            });
        }
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

fn poll(fd: RawFd, timeout: Option<Duration>) -> io::Result<()> {
    let mut fds = [libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    }];
    let res = unsafe {
        libc::poll(
            fds.as_mut_ptr(),
            1,
            timeout.map_or(-1, |t| t.as_millis() as _),
        )
    };
    match res {
        -1 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

fn exec(cmd: &str) {
    let mut proc = Command::new("sh");
    proc.args(["-c", cmd]);
    proc.stdin(Stdio::null());
    proc.stdout(Stdio::null());
    // Safety: libc::daemon() is async-signal-safe
    unsafe {
        proc.pre_exec(|| match libc::daemon(1, 0) {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
    proc.spawn().unwrap().wait().unwrap();
}

#[derive(Clone, Copy)]
struct XkbMaskState {
    depressed_mods: u32,
    latched_mods: u32,
    locked_mods: u32,
    depressed_layout: u32,
    latched_layout: u32,
    locked_layout: u32,
}

impl XkbMaskState {
    fn new(xkb_state: &xkb::State) -> Self {
        Self {
            depressed_mods: xkb_state.serialize_mods(xkb::STATE_MODS_DEPRESSED),
            latched_mods: xkb_state.serialize_mods(xkb::STATE_MODS_LATCHED),
            locked_mods: xkb_state.serialize_mods(xkb::STATE_MODS_LOCKED),
            depressed_layout: xkb_state.serialize_layout(xkb::STATE_LAYOUT_DEPRESSED),
            latched_layout: xkb_state.serialize_layout(xkb::STATE_LAYOUT_LATCHED),
            locked_layout: xkb_state.serialize_layout(xkb::STATE_LAYOUT_LOCKED),
        }
    }

    fn with_locked_layout(&self, locked_layout: u32) -> Self {
        Self {
            locked_layout,
            ..*self
        }
    }

    fn apply(&self, xkb_state: &xkb::State) {
        // Hack: this is just ref counting, no actual cloning. `update_mask` should probably just
        // accept `&self` instead of `&mut self`.
        let mut xkb_state = xkb_state.clone();
        xkb_state.update_mask(
            self.depressed_mods,
            self.latched_mods,
            self.locked_mods,
            self.depressed_layout,
            self.latched_layout,
            self.locked_layout,
        );
    }
}
