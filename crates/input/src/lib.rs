#[cfg(windows)]
mod devices;
#[cfg(windows)]
mod raw_input;

#[cfg(windows)]
pub use devices::keyboards;
#[cfg(windows)]
pub use raw_input::{Capture, Registration, registrations};

use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    // The key's position as a set-1 scan code, with 0xE0 or 0xE1 in the high byte for prefixed keys.
    pub scan: u16,
    pub up: bool,
    // The Raw Input device handle. SendInput arrives as 0, and so does a user-mode remapper built
    // on it. Remaps below user mode, such as a Scancode Map, keep the real device.
    pub device: isize,
    // When the app read the event. Raw Input carries no timestamp of its own, so this includes the
    // time the message waited in the queue.
    pub at: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keyboard {
    // The Raw Input device handle, the value KeyEvent::device carries.
    pub handle: isize,
    pub path: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    // Windows gives each external device one container ID, shared by all its HID collections, so it
    // groups one keyboard's collections and tells two identical keyboards apart. Everything Windows
    // counts as part of the computer, such as a laptop's own keyboard and its hotkey collections,
    // shares the null container {00000000-0000-0000-ffff-ffffffffffff}. Events carry `handle`.
    pub container: Option<u128>,
}

impl Keyboard {
    // Many product strings already start with the maker's name, and some keyboards report none.
    pub fn name(&self) -> String {
        match (&self.manufacturer, &self.product) {
            (Some(maker), Some(product)) if !product.starts_with(maker.as_str()) => {
                format!("{maker} {product}")
            }
            (_, Some(product)) => product.clone(),
            (Some(maker), None) => format!("{maker} keyboard"),
            (None, None) => match (self.vendor_id, self.product_id) {
                (Some(vendor), Some(product)) => format!("Keyboard {vendor:04x}:{product:04x}"),
                _ => "Keyboard".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyboard(maker: Option<&str>, product: Option<&str>, ids: Option<(u16, u16)>) -> Keyboard {
        Keyboard {
            handle: 1,
            path: String::new(),
            vendor_id: ids.map(|i| i.0),
            product_id: ids.map(|i| i.1),
            manufacturer: maker.map(str::to_string),
            product: product.map(str::to_string),
            container: None,
        }
    }

    #[test]
    fn names_a_keyboard_from_what_it_reports() {
        assert_eq!(
            keyboard(Some("Keychron"), Some("K2"), None).name(),
            "Keychron K2"
        );
        assert_eq!(
            keyboard(Some("Logitech"), Some("Logitech USB Receiver"), None).name(),
            "Logitech USB Receiver"
        );
        assert_eq!(
            keyboard(None, Some("USB Keyboard"), None).name(),
            "USB Keyboard"
        );
        assert_eq!(keyboard(Some("Razer"), None, None).name(), "Razer keyboard");
        assert_eq!(
            keyboard(None, None, Some((0x046d, 0xc52b))).name(),
            "Keyboard 046d:c52b"
        );
        assert_eq!(keyboard(None, None, None).name(), "Keyboard");
    }
}
