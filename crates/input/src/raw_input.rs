use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::size_of;
use std::rc::Rc;

use windows::Win32::Foundation::{E_FAIL, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::{
    GetRawInputData, GetRegisteredRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTDEVICE_FLAGS, RAWINPUTHEADER, RAWKEYBOARD, RID_INPUT, RIDEV_REMOVE, RIM_TYPEKEYBOARD,
    RegisterRawInputDevices,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass,
};
use windows::Win32::UI::WindowsAndMessaging::{
    RI_KEY_BREAK, RI_KEY_E0, RI_KEY_E1, RIM_INPUT, WM_INPUT, WM_NCDESTROY,
};

use crate::KeyEvent;

const GENERIC_DESKTOP: u16 = 0x01;
const KEYBOARD: u16 = 0x06;
const SUBCLASS_ID: usize = 0x6b74;

type OnKey = RefCell<Box<dyn FnMut(KeyEvent)>>;

pub struct Capture {
    hwnd: HWND,
    on_key: *const OnKey,
}

impl Capture {
    // Call this on the thread that owns `hwnd`. comctl32 cannot subclass a window across threads.
    pub fn start(
        hwnd: isize,
        on_key: impl FnMut(KeyEvent) + 'static,
    ) -> windows::core::Result<Self> {
        let hwnd = HWND(hwnd as *mut c_void);
        let on_key: Box<dyn FnMut(KeyEvent)> = Box::new(on_key);
        let capture = Capture {
            hwnd,
            on_key: Rc::into_raw(Rc::new(RefCell::new(on_key))),
        };
        let subclassed = unsafe {
            SetWindowSubclass(
                hwnd,
                Some(subclass_proc),
                SUBCLASS_ID,
                capture.on_key as usize,
            )
        };
        // comctl32 sets no error code when this fails.
        if !subclassed.as_bool() {
            return Err(windows::core::Error::new(
                E_FAIL,
                "SetWindowSubclass failed: the window is invalid or belongs to another thread",
            ));
        }
        // No sink flag, so Windows sends WM_INPUT only while this process owns the foreground
        // window. The target must be set: a null target follows keyboard focus, and focus sits in
        // WebView2's own process.
        let keyboard = RAWINPUTDEVICE {
            usUsagePage: GENERIC_DESKTOP,
            usUsage: KEYBOARD,
            dwFlags: RAWINPUTDEVICE_FLAGS(0),
            hwndTarget: hwnd,
        };
        unsafe { RegisterRawInputDevices(&[keyboard], size_of::<RAWINPUTDEVICE>() as u32) }?;
        Ok(capture)
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        // Removal requires a null target.
        let keyboard = RAWINPUTDEVICE {
            usUsagePage: GENERIC_DESKTOP,
            usUsage: KEYBOARD,
            dwFlags: RIDEV_REMOVE,
            hwndTarget: HWND::default(),
        };
        let _ = unsafe { RegisterRawInputDevices(&[keyboard], size_of::<RAWINPUTDEVICE>() as u32) };
        // The window may be gone and its handle reused, so only remove a subclass that is still ours.
        let mut data = 0;
        let attached = unsafe {
            GetWindowSubclass(self.hwnd, Some(subclass_proc), SUBCLASS_ID, Some(&mut data))
        };
        if attached.as_bool() && data == self.on_key as usize {
            let _ = unsafe { RemoveWindowSubclass(self.hwnd, Some(subclass_proc), SUBCLASS_ID) };
        }
        drop(unsafe { Rc::from_raw(self.on_key) });
    }
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    on_key: usize,
) -> LRESULT {
    match msg {
        // Nothing registers a sink, so only RIM_INPUT can arrive: input made while this process
        // was in the foreground. Any other code is never read.
        WM_INPUT if wparam.0 & 0xff == RIM_INPUT as usize => {
            if let Some(event) = read(HRAWINPUT(lparam.0 as *mut c_void)) {
                // The handler may drop the Capture, so this call holds its own reference until
                // the handler returns.
                let handler = unsafe {
                    Rc::increment_strong_count(on_key as *const OnKey);
                    Rc::from_raw(on_key as *const OnKey)
                };
                // A handler that pumps messages would re-enter here. Its events are dropped
                // rather than handed to a second mutable borrow.
                if let Ok(mut on_key) = handler.try_borrow_mut() {
                    on_key(event);
                }
            }
        }
        WM_NCDESTROY => {
            let _ = unsafe { RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID) };
        }
        _ => {}
    }
    // The chain ends in DefWindowProc, which frees the input for RIM_INPUT.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

// The handle is only valid inside this WM_INPUT, so it is read here and never passed on.
fn read(input: HRAWINPUT) -> Option<KeyEvent> {
    let mut raw = RAWINPUT::default();
    let mut size = size_of::<RAWINPUT>() as u32;
    let copied = unsafe {
        GetRawInputData(
            input,
            RID_INPUT,
            Some((&mut raw as *mut RAWINPUT).cast()),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if copied == u32::MAX || raw.header.dwType != RIM_TYPEKEYBOARD.0 {
        return None;
    }
    Some(key_event(unsafe { &raw.data.keyboard }, raw.header.hDevice))
}

fn key_event(keyboard: &RAWKEYBOARD, device: HANDLE) -> KeyEvent {
    // Flags also carries terminal server bits, so each bit is tested on its own.
    let flags = u32::from(keyboard.Flags);
    let prefix = if flags & RI_KEY_E1 != 0 {
        0xE100
    } else if flags & RI_KEY_E0 != 0 {
        0xE000
    } else {
        0
    };
    KeyEvent {
        scan: prefix | keyboard.MakeCode,
        up: flags & RI_KEY_BREAK != 0,
        device: device.0 as isize,
    }
}

pub struct Registration {
    pub usage_page: u16,
    pub usage: u16,
    pub flags: u32,
    pub target: isize,
}

// Every Raw Input registration in this process. Registration is per process and the last call
// wins, so this is how a check proves that capture has no sink flag and targets the app window.
pub fn registrations() -> windows::core::Result<Vec<Registration>> {
    let item = size_of::<RAWINPUTDEVICE>() as u32;
    let mut count = 0;
    // Without a buffer the call fails by design and reports how many there are.
    unsafe { GetRegisteredRawInputDevices(None, &mut count, item) };
    if count == 0 {
        return Ok(Vec::new());
    }
    let mut devices = vec![RAWINPUTDEVICE::default(); count as usize];
    let written =
        unsafe { GetRegisteredRawInputDevices(Some(devices.as_mut_ptr()), &mut count, item) };
    if written == u32::MAX {
        return Err(windows::core::Error::from_thread());
    }
    devices.truncate(written as usize);
    Ok(devices
        .iter()
        .map(|d| Registration {
            usage_page: d.usUsagePage,
            usage: d.usUsage,
            flags: d.dwFlags.0,
            target: d.hwndTarget.0 as isize,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(make_code: u16, flags: u32) -> KeyEvent {
        let keyboard = RAWKEYBOARD {
            MakeCode: make_code,
            Flags: flags as u16,
            ..Default::default()
        };
        key_event(&keyboard, HANDLE(0x1234 as *mut c_void))
    }

    #[test]
    fn reads_a_plain_key() {
        let down = key(0x1e, 0);
        assert_eq!((down.scan, down.up, down.device), (0x1e, false, 0x1234));
        assert!(key(0x1e, RI_KEY_BREAK).up);
    }

    #[test]
    fn puts_the_prefix_in_the_high_byte() {
        assert_eq!(key(0x1d, RI_KEY_E0 | RI_KEY_BREAK).scan, 0xe01d);
        assert_eq!(key(0x1d, RI_KEY_E1).scan, 0xe11d);
    }

    #[test]
    fn ignores_terminal_server_bits() {
        let down = key(0x1e, 0x08 | 0x10);
        assert_eq!((down.scan, down.up), (0x1e, false));
    }
}
