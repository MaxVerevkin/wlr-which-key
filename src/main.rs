#[macro_use]
extern crate log;

mod color;
mod config;
mod menu;
mod text;

use std::{
    f64::consts::{FRAC_PI_2, PI, TAU},
    io,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::client::{
        protocol::{wl_keyboard, wl_output, wl_seat, wl_shm, wl_surface},
        Connection, QueueHandle,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{keysyms, KeyEvent, KeyboardHandler, Modifiers},
        Capability, SeatHandler, SeatState,
    },
    shell::layer::{
        KeyboardInteractivity, Layer, LayerHandler, LayerState, LayerSurface, LayerSurfaceConfigure,
    },
    shm::{slot::SlotPool, ShmHandler, ShmState},
};

use pangocairo::cairo;

fn main() {
    env_logger::init();

    let config = config::Config::new().unwrap();
    if config.menu.0.is_empty() {
        return;
    }

    let conn = Connection::connect_to_env().unwrap();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let menu = menu::Menu::new(&config.font, &config.menu);

    let mut state = State {
        registry_state: RegistryState::new(&conn, &qh),
        seat_state: SeatState::new(),
        output_state: OutputState::new(),
        compositor_state: CompositorState::new(),
        shm_state: ShmState::new(),
        layer_state: LayerState::new(),

        exit: false,
        configured: false,
        pool: None,
        width: (menu.width() + config.corner_r * 2.0) as u32,
        height: (menu.height() + config.corner_r * 2.0) as u32,
        scale: 1,
        layer: None,
        keyboards: Vec::new(),
        menu,
        config,
    };

    while !state.registry_state.ready() {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }

    let pool = SlotPool::new(
        state.width as usize * state.height as usize * 4,
        &state.shm_state,
    )
    .expect("Failed to create pool");
    state.pool = Some(pool);

    let surface = state.compositor_state.create_surface(&qh).unwrap();
    let layer = LayerSurface::builder()
        .size((state.width, state.height))
        .keyboard_interactivity(KeyboardInteractivity::Exclusive)
        .namespace("wlr_which_key")
        .map(&qh, &mut state.layer_state, surface, Layer::Top)
        .expect("layer surface creation");
    state.layer = Some(layer);

    while !state.exit {
        event_queue
            .blocking_dispatch(&mut state)
            .expect("dispatching");
    }
}

struct State {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: ShmState,
    layer_state: LayerState,

    exit: bool,
    configured: bool,
    pool: Option<SlotPool>,
    width: u32,
    height: u32,
    scale: i32,
    layer: Option<LayerSurface>,
    keyboards: Vec<(wl_seat::WlSeat, wl_keyboard::WlKeyboard)>,
    menu: menu::Menu,
    config: config::Config,
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        scale: i32,
    ) {
        self.scale = scale;
        self.draw();
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerHandler for State {
    fn layer_state(&mut self) -> &mut LayerState {
        &mut self.layer_state
    }

    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if configure.new_size.0 != 0 {
            self.width = configure.new_size.0;
        }
        if configure.new_size.1 != 0 {
            self.height = configure.new_size.1;
        }

        self.configured = true;
        self.draw();
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            let kbd = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to get keyboard");
            self.keyboards.push((seat, kbd));
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            self.keyboards.retain(|(s, _)| *s != seat);
        }
    }
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[u32],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if event.keysym == keysyms::XKB_KEY_Escape {
            self.exit = true;
            return;
        }

        if let Some(key) = event.utf8 && let Some(action) = self.menu.get_action(&key) {
            match action {
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
                menu::Action::Submenu(new_menu) => {
                    let layer = self.layer.as_mut().expect("no layer?");
                    self.width = (new_menu.width() + self.config.corner_r * 2.0) as u32;
                    self.height = (new_menu.height() + self.config.corner_r * 2.0) as u32;
                    self.menu = new_menu;
                    layer.set_size(self.width, self.height);
                    layer.wl_surface().commit();
                }
            }
        }
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: Modifiers,
    ) {
    }

    fn update_repeat_info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: smithay_client_toolkit::seat::keyboard::RepeatInfo,
    ) {
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

impl State {
    pub fn draw(&mut self) {
        if !self.configured {
            return;
        }

        let layer = self.layer.as_mut().expect("no layer?");
        let pool = self.pool.as_mut().expect("no pool?");
        let stride = self.width as i32 * 4;
        let width_f = self.width as f64;
        let height_f = self.height as f64;

        let (buffer, canvas) = pool
            .create_buffer(
                self.width as i32 * self.scale,
                self.height as i32 * self.scale,
                stride * self.scale,
                wl_shm::Format::Argb8888,
            )
            .expect("create buffer");

        let cairo_surf = unsafe {
            cairo::ImageSurface::create_for_data_unsafe(
                canvas.as_mut_ptr(),
                cairo::Format::ARgb32,
                self.width as i32 * self.scale,
                self.height as i32 * self.scale,
                stride * self.scale,
            )
            .expect("cairo surface")
        };

        let cairo_ctx = cairo::Context::new(&cairo_surf).expect("cairo context");
        cairo_ctx.scale(self.scale as f64, self.scale as f64);
        layer.wl_surface().set_buffer_scale(self.scale);

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
        layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

        // Attach and commit to present.
        buffer.attach_to(layer.wl_surface()).expect("buffer attach");
        layer.wl_surface().commit();
    }
}

delegate_compositor!(State);
delegate_output!(State);
delegate_shm!(State);
delegate_seat!(State);
delegate_keyboard!(State);
delegate_layer!(State);
delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![
        CompositorState,
        OutputState,
        ShmState,
        SeatState,
        LayerState,
    ];
}
