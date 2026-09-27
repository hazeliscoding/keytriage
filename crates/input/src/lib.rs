#[cfg(windows)]
mod raw_input;

#[cfg(windows)]
pub use raw_input::{Capture, Registration, registrations};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    // The key's position as a set-1 scan code, with 0xE0 or 0xE1 in the high byte for prefixed keys.
    pub scan: u16,
    pub up: bool,
    // The Raw Input device handle. SendInput arrives as 0, and so does a user-mode remapper built
    // on it. Remaps below user mode, such as a Scancode Map, keep the real device.
    pub device: isize,
}
