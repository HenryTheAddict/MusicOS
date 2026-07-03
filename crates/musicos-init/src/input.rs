#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Confirm,
    Back,
    Up,
    Down,
    Left,
    Right,
    Context,
    Home,
    Menu,
    Sleep,
    VolumeUp,
    VolumeDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Button(Button),
    ButtonRelease(Button),
    AnyButton,
    Swipe(SwipeDirection),
    Tap,
    Power,
    WifiSync,
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use evdev::{AbsoluteAxisType, Device, EventType, InputEventKind, Key};
    use std::os::fd::AsRawFd;
    use std::time::{Duration, Instant};

    pub struct InputSystem {
        devices: Vec<Device>,
        pointer: PointerTracker,
        pressed_buttons: std::collections::HashSet<Button>,
        pressed_keys: std::collections::HashSet<u16>,
    }

    impl InputSystem {
        pub fn open() -> Self {
            let mut devices = Vec::new();
            if let Ok(entries) = std::fs::read_dir("/dev/input") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .starts_with("event")
                    {
                        continue;
                    }
                    match Device::open(&path) {
                        Ok(device) => {
                            let _ = set_nonblocking(&device);
                            log::info!("opened input device {}", path.display());
                            devices.push(device);
                        }
                        Err(err) => log::info!("skipping input device {}: {err}", path.display()),
                    }
                }
            }

            Self {
                devices,
                pointer: PointerTracker::default(),
                pressed_buttons: std::collections::HashSet::new(),
                pressed_keys: std::collections::HashSet::new(),
            }
        }

        pub fn poll(&mut self) -> Vec<InputEvent> {
            let mut out = Vec::new();
            for device in &mut self.devices {
                let Ok(events) = device.fetch_events() else {
                    continue;
                };
                for event in events {
                    match event.kind() {
                        InputEventKind::Key(key) => {
                            let value = event.value();
                            let code = key.code();
                            log_input(&format!("key {:?} code={} value={}", key, code, value));
                            if value == 0 {
                                self.pressed_keys.remove(&code);
                            } else {
                                self.pressed_keys.insert(code);

                                // A + Start: BTN_SOUTH (304) + BTN_START (315)
                                if self.pressed_keys.contains(&304)
                                    && self.pressed_keys.contains(&315)
                                {
                                    log::info!("debug wifi sync chorded: A + Start");
                                    out.push(InputEvent::WifiSync);
                                }
                            }

                            if let Some(button) = map_key_to_button(key) {
                                if value == 0 {
                                    self.pressed_buttons.remove(&button);
                                    out.push(InputEvent::ButtonRelease(button));
                                } else {
                                    self.pressed_buttons.insert(button);

                                    // Chord: VolumeDown + Home -> Menu (Open options/diagnostics)
                                    if self.pressed_buttons.contains(&Button::VolumeDown)
                                        && self.pressed_buttons.contains(&Button::Home)
                                    {
                                        log::info!("diagnostic chord chorded: VolumeDown + Home");
                                        out.push(InputEvent::Button(Button::Menu));
                                    }

                                    out.push(InputEvent::Button(button));
                                }
                            } else if key == Key::KEY_POWER && value != 0 {
                                out.push(InputEvent::Power);
                            }
                        }
                        InputEventKind::AbsAxis(axis) => {
                            let value = event.value();
                            if let Some(event) = map_abs_dpad(axis, value) {
                                log_input(&format!(
                                    "axis {:?} value={} -> {:?}",
                                    axis, value, event
                                ));
                                out.push(event);
                            } else if let Some(event) = self.pointer.update_abs(axis, value) {
                                out.push(event);
                            }
                        }
                        InputEventKind::RelAxis(_) if event.event_type() == EventType::RELATIVE => {
                        }
                        _ => {}
                    }
                }
            }
            out
        }
    }

    fn set_nonblocking(device: &Device) -> std::io::Result<()> {
        let fd = device.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if rc < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    fn map_key_to_button(key: Key) -> Option<Button> {
        Some(match key {
            Key::KEY_ENTER
            | Key::KEY_SPACE
            | Key::KEY_OK
            | Key::KEY_PLAYPAUSE
            | Key::KEY_SELECT
            | Key::BTN_SOUTH
            | Key::BTN_SELECT
            | Key::BTN_START
            | Key::BTN_TOUCH
            | Key::BTN_LEFT => Button::Confirm,
            Key::KEY_ESC | Key::KEY_BACK | Key::KEY_BACKSPACE | Key::BTN_EAST | Key::BTN_TL => {
                Button::Back
            }
            Key::KEY_X | Key::BTN_NORTH | Key::BTN_TR => Button::Context,
            Key::KEY_Y | Key::BTN_WEST => Button::Home,
            Key::KEY_MENU | Key::KEY_HOMEPAGE | Key::BTN_MODE | Key::BTN_TRIGGER => Button::Menu,
            Key::KEY_SLEEP | Key::KEY_SUSPEND => Button::Sleep,
            Key::KEY_VOLUMEUP => Button::VolumeUp,
            Key::KEY_VOLUMEDOWN => Button::VolumeDown,
            Key::KEY_UP | Key::BTN_DPAD_UP => Button::Up,
            Key::KEY_DOWN | Key::BTN_DPAD_DOWN => Button::Down,
            Key::KEY_LEFT | Key::KEY_PREVIOUSSONG | Key::BTN_DPAD_LEFT => Button::Left,
            Key::KEY_RIGHT | Key::KEY_NEXTSONG | Key::BTN_DPAD_RIGHT => Button::Right,
            _ => return None,
        })
    }

    fn map_abs_dpad(axis: AbsoluteAxisType, value: i32) -> Option<InputEvent> {
        match axis {
            AbsoluteAxisType::ABS_HAT0X
            | AbsoluteAxisType::ABS_HAT1X
            | AbsoluteAxisType::ABS_HAT2X
            | AbsoluteAxisType::ABS_HAT3X => match value {
                v if v < 0 => Some(InputEvent::Button(Button::Left)),
                v if v > 0 => Some(InputEvent::Button(Button::Right)),
                _ => None,
            },
            AbsoluteAxisType::ABS_HAT0Y
            | AbsoluteAxisType::ABS_HAT1Y
            | AbsoluteAxisType::ABS_HAT2Y
            | AbsoluteAxisType::ABS_HAT3Y => match value {
                v if v < 0 => Some(InputEvent::Button(Button::Up)),
                v if v > 0 => Some(InputEvent::Button(Button::Down)),
                _ => None,
            },
            AbsoluteAxisType::ABS_X => match value {
                v if v < -16_000 => Some(InputEvent::Button(Button::Left)),
                v if v > 16_000 => Some(InputEvent::Button(Button::Right)),
                _ => None,
            },
            AbsoluteAxisType::ABS_Y => match value {
                v if v < -16_000 => Some(InputEvent::Button(Button::Up)),
                v if v > 16_000 => Some(InputEvent::Button(Button::Down)),
                _ => None,
            },
            _ => None,
        }
    }

    fn log_input(line: &str) {
        log::info!("input {line}");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/boot/musicos-input.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "{line}");
        }
    }

    #[derive(Default)]
    struct PointerTracker {
        start: Option<(i32, i32, Instant)>,
        last_x: i32,
        last_y: i32,
    }

    impl PointerTracker {
        fn update_abs(&mut self, axis: AbsoluteAxisType, value: i32) -> Option<InputEvent> {
            match axis {
                AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_MT_POSITION_X => {
                    self.last_x = value
                }
                AbsoluteAxisType::ABS_Y | AbsoluteAxisType::ABS_MT_POSITION_Y => {
                    self.last_y = value
                }
                AbsoluteAxisType::ABS_PRESSURE | AbsoluteAxisType::ABS_MT_PRESSURE => {
                    if value > 0 && self.start.is_none() {
                        self.start = Some((self.last_x, self.last_y, Instant::now()));
                    } else if value == 0 {
                        return self.finish();
                    }
                }
                _ => {}
            }
            None
        }

        fn finish(&mut self) -> Option<InputEvent> {
            let (x, y, started) = self.start.take()?;
            let dx = self.last_x - x;
            let dy = self.last_y - y;
            if started.elapsed() < Duration::from_millis(250) && dx.abs() < 25 && dy.abs() < 25 {
                return Some(InputEvent::Tap);
            }
            if dx.abs() > dy.abs() && dx.abs() > 80 {
                return Some(InputEvent::Swipe(if dx > 0 {
                    SwipeDirection::Right
                } else {
                    SwipeDirection::Left
                }));
            }
            if dy.abs() > 80 {
                return Some(InputEvent::Swipe(if dy > 0 {
                    SwipeDirection::Down
                } else {
                    SwipeDirection::Up
                }));
            }
            None
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod linux {
    pub struct InputSystem;

    impl InputSystem {
        pub fn open() -> Self {
            Self
        }

        pub fn poll(&mut self) -> Vec<super::InputEvent> {
            Vec::new()
        }
    }
}

pub use linux::InputSystem;
