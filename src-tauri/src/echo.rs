// Debug builds only. With KEYTRIAGE_ECHO set, the page starts a test once it has loaded and each of
// the test's events prints, and page loads and WebView2's browser settings print too, so the checks
// in scripts/ can prove where input stops and that browser keys do nothing. Only the checks' marker
// keys, F13 to F15, print their scan code, so real typing never shows up in a terminal or a CI log.
use std::ffi::c_void;
use std::io::Write;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};

use keytriage_input::{keyboards, registrations};
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{Listener, Runtime, Webview, WebviewWindow};

use crate::session_core::Entry;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2Settings3,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Diagnostics::Debug::GetErrorMode;
use windows::Win32::UI::Input::{RAWINPUTDEVICE, RAWINPUTDEVICE_FLAGS, RegisterRawInputDevices};
use windows_core::{BOOL, Interface};

fn enabled() -> bool {
    std::env::var_os("KEYTRIAGE_ECHO").is_some()
}

pub fn note(line: &str) {
    if enabled() {
        print(line);
    }
}

pub fn start<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), Box<dyn std::error::Error>> {
    if !enabled() {
        return Ok(());
    }
    window.listen_any("test:started", |_| {
        print_registrations();
        print("kt-input: ready");
    });
    window.listen_any("test:stopped", |_| print("kt-input: stopped"));
    let page = window.clone();
    let hwnd = window.hwnd()?.0 as isize;
    window.listen_any("test:event", move |event| {
        match serde_json::from_str::<Entry>(event.payload()) {
            Ok(Entry::Key {
                scan, up, device, ..
            }) => {
                print(&line(scan, up, device));
                // F16 makes the page reload itself, a navigation that only the navigation guard
                // can refuse, for the browser keys check.
                if scan == 0x67 && !up {
                    let _ = page.eval("location.reload()");
                }
                // F17 makes the page pause the test and continue it 3 s later, as the Pause and
                // Continue buttons do, for the focus check's user pause.
                if scan == 0x68 && !up {
                    let _ = page.eval(
                        "window.__TAURI_INTERNALS__.invoke('pause_test').then(() => setTimeout(() => \
                         window.__TAURI_INTERNALS__.invoke('continue_test'), 3000))",
                    );
                }
            }
            Ok(Entry::Paused { interrupted, .. }) => {
                if crate::positive_control("KEYTRIAGE_KEEP_REGISTRATION") {
                    keep_registration(hwnd);
                }
                // A count that can't be read must not look like zero.
                let count =
                    registrations().map_or("unreadable".to_string(), |r| r.len().to_string());
                print(&format!(
                    "kt-input: paused registrations={count} interrupted={}",
                    interrupted.len()
                ));
            }
            Ok(Entry::Resumed { .. }) => {
                print("kt-input: resumed");
                print_registrations();
            }
            Err(_) => {}
        }
    });
    for k in keyboards()? {
        print(&format!(
            "kt-input: keyboard handle=0x{:x} container={} vid={} pid={} name={:?}",
            k.handle,
            k.container.map_or("-".to_string(), |c| format!("{c:032x}")),
            k.vendor_id.map_or("-".to_string(), |v| format!("{v:04x}")),
            k.product_id.map_or("-".to_string(), |p| format!("{p:04x}")),
            k.name()
        ));
    }
    print(&format!("kt-shell: error-mode=0x{:x}", unsafe {
        GetErrorMode()
    }));
    // with_webview calls run in order, so this reads the settings after browser_ui changed them.
    window.with_webview(|webview| print(&settings_line(&webview.controller())))?;
    Ok(())
}

// Each snapshot starts with its count, so a reader can drop the one before.
fn print_registrations() {
    let all = registrations().unwrap_or_default();
    print(&format!("kt-input: registrations n={}", all.len()));
    for r in all {
        print(&format!(
            "kt-input: registered page=0x{:x} usage=0x{:x} flags=0x{:x} target=0x{:x}",
            r.usage_page, r.usage, r.flags, r.target
        ));
    }
}

// The test starts the way the UI will start it: the page calls the command.
static STARTED: AtomicBool = AtomicBool::new(false);

// A refused navigation still finishes, so loads are counted as they start.
pub fn page_load<R: Runtime>(webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
    if !enabled() {
        return;
    }
    if payload.event() == PageLoadEvent::Started {
        print("kt-shell: page-load");
    } else if !STARTED.load(Ordering::SeqCst)
        && webview
            .eval("window.__TAURI_INTERNALS__.invoke('start_test')")
            .is_ok()
    {
        STARTED.store(true, Ordering::SeqCst);
        print("kt-shell: start-test sent");
    }
}

// The focus check's positive control for "no registration while paused": the keyboard is
// registered again on the app window, still without a sink flag, as a paused capture must not.
fn keep_registration(hwnd: isize) {
    let keyboard = RAWINPUTDEVICE {
        usUsagePage: 0x01,
        usUsage: 0x06,
        dwFlags: RAWINPUTDEVICE_FLAGS(0),
        hwndTarget: HWND(hwnd as *mut c_void),
    };
    let _ = unsafe { RegisterRawInputDevices(&[keyboard], size_of::<RAWINPUTDEVICE>() as u32) };
}

fn settings_line(controller: &ICoreWebView2Controller) -> String {
    match browser_settings(controller) {
        Ok((keys, menus)) => format!(
            "kt-shell: browser-keys={} context-menus={}",
            u8::from(keys),
            u8::from(menus)
        ),
        Err(e) => format!("kt-shell: settings unreadable: {e}"),
    }
}

fn browser_settings(controller: &ICoreWebView2Controller) -> windows_core::Result<(bool, bool)> {
    let (mut keys, mut menus) = (BOOL::default(), BOOL::default());
    unsafe {
        let settings = controller.CoreWebView2()?.Settings()?;
        settings.AreDefaultContextMenusEnabled(&mut menus)?;
        settings
            .cast::<ICoreWebView2Settings3>()?
            .AreBrowserAcceleratorKeysEnabled(&mut keys)?;
    }
    Ok((keys.as_bool(), menus.as_bool()))
}

// A closed pipe must not panic inside the window procedure.
fn print(line: &str) {
    let _ = writeln!(std::io::stdout(), "{line}");
}

fn line(scan: u16, up: bool, device: isize) -> String {
    let key = match scan {
        0x64..=0x66 => format!("0x{scan:x}"),
        _ => "other".to_string(),
    };
    format!(
        "kt-input: key={key} up={} device=0x{device:x}",
        u8::from(up)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_the_marker_keys() {
        assert_eq!(
            line(0x64, false, 0x2a),
            "kt-input: key=0x64 up=0 device=0x2a"
        );
        assert_eq!(
            line(0x66, true, 0x2a),
            "kt-input: key=0x66 up=1 device=0x2a"
        );
    }

    #[test]
    fn hides_every_other_key() {
        for scan in [0x1e, 0x63, 0x67, 0x68, 0xe01d, 0xe11d] {
            assert_eq!(
                line(scan, false, 0x2a),
                "kt-input: key=other up=0 device=0x2a"
            );
        }
    }
}
