use std::os::fd::AsRawFd;
use std::time::Duration;

use wayrs_client::connection::Connection;
use wayrs_client::protocol::*;
use wayrs_client::proxy::Proxy;

use xkbcommon::xkb;

pub trait KeyboardHandler: Sized + 'static {
    fn keyboard(&mut self, wl_keyboard: WlKeyboard) -> Option<&mut Keyboard>;

    fn repeat_info(
        &mut self,
        conn: &mut Connection<Self>,
        wl_keyboard: WlKeyboard,
        info: RepeatInfo,
    );

    fn key_pressed(
        &mut self,
        conn: &mut Connection<Self>,
        wl_keyboard: WlKeyboard,
        xkb: xkb::State,
        key_code: xkb::Keycode,
    );

    fn key_released(
        &mut self,
        conn: &mut Connection<Self>,
        wl_keyboard: WlKeyboard,
        xkb: xkb::State,
        key_code: xkb::Keycode,
    );
}

pub struct Keyboard {
    pub wl_seat: WlSeat,
    pub wl_keyboard: WlKeyboard,
    pub xkb_context: xkb::Context,
    pub xkb_state: Option<xkb::State>,
}

#[derive(Debug, Clone, Copy)]
pub struct RepeatInfo {
    pub delay: Duration,
    pub interval: Duration,
}

impl Keyboard {
    pub fn new<D: KeyboardHandler>(conn: &mut Connection<D>, wl_seat: WlSeat) -> Self {
        Self {
            wl_seat,
            wl_keyboard: wl_seat.get_keyboard_with_cb(conn, wl_keyboard_cb),
            xkb_context: xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
            xkb_state: None,
        }
    }

    pub fn release<D>(self, conn: &mut Connection<D>) {
        if self.wl_keyboard.version() >= 3 {
            self.wl_keyboard.release(conn)
        }
    }
}

fn wl_keyboard_cb<D: KeyboardHandler>(
    conn: &mut Connection<D>,
    state: &mut D,
    wl_keyboard: WlKeyboard,
    event: wl_keyboard::Event,
) {
    let Some(keyboard) = state.keyboard(wl_keyboard)
    else { return };

    match event {
        wl_keyboard::Event::Keymap(args) => {
            if args.format != wl_keyboard::KeymapFormat::XkbV1 {
                eprintln!("unsupported wl_keyboard keymap format");
                return;
            }

            let keymap = unsafe {
                xkb::Keymap::new_from_fd(
                    &keyboard.xkb_context,
                    args.fd.as_raw_fd(),
                    args.size as usize,
                    xkb::FORMAT_TEXT_V1,
                    xkb::KEYMAP_COMPILE_NO_FLAGS,
                )
            };

            match keymap {
                Ok(Some(keymap)) => keyboard.xkb_state = Some(xkb::State::new(&keymap)),
                Ok(None) => eprintln!("could not create keymap"),
                Err(e) => eprintln!("could not create keymap: {e}"),
            }
        }
        wl_keyboard::Event::Enter(_) => (),
        wl_keyboard::Event::Leave(_) => (),
        wl_keyboard::Event::Key(args) => {
            if let Some(xkb_state) = keyboard.xkb_state.clone() {
                match args.state {
                    wl_keyboard::KeyState::Released => {
                        state.key_released(conn, wl_keyboard, xkb_state, args.key + 8);
                    }
                    wl_keyboard::KeyState::Pressed => {
                        state.key_pressed(conn, wl_keyboard, xkb_state, args.key + 8);
                    }
                }
            }
        }
        wl_keyboard::Event::Modifiers(args) => {
            if let Some(xkb_state) = &mut keyboard.xkb_state {
                xkb_state.update_mask(
                    args.mods_depressed,
                    args.mods_latched,
                    args.mods_locked,
                    0,
                    0,
                    args.group,
                );
            }
        }
        wl_keyboard::Event::RepeatInfo(args) => {
            let rate: u64 = args.rate.try_into().unwrap();
            let delay: u64 = args.delay.try_into().unwrap();
            state.repeat_info(
                conn,
                wl_keyboard,
                RepeatInfo {
                    delay: Duration::from_millis(delay),
                    interval: Duration::from_micros(1_000_000u64 / rate),
                },
            );
        }
    }
}
