// Debug builds only. With KEYTRIAGE_ECHO set, the page starts a test once it has loaded and each of
// the test's events prints, and page loads and WebView2's browser settings print too, so the checks
// in scripts/ can prove where input stops and that browser keys do nothing. Only the checks' marker
// keys, F13 to F15, print their scan code, so real typing never shows up in a terminal or a CI log.
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use keytriage_input::{keyboards, registrations};
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{Listener, Runtime, Webview, WebviewWindow};

use crate::test_session::Entry;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2Settings3,
};
use windows::Win32::System::Diagnostics::Debug::GetErrorMode;
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
    window.listen_any("test:event", |event| {
        match serde_json::from_str::<Entry>(event.payload()) {
            Ok(Entry::Key {
                scan, up, device, ..
            }) => print(&line(scan, up, device)),
            Ok(Entry::Paused { interrupted, .. }) => print(&format!(
                "kt-input: paused registrations={} interrupted={}",
                registrations().map_or(0, |r| r.len()),
                interrupted.len()
            )),
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

pub fn page_load<R: Runtime>(webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
    if !enabled() || payload.event() != PageLoadEvent::Finished {
        return;
    }
    print("kt-shell: page-load");
    if !STARTED.swap(true, Ordering::SeqCst) {
        let _ = webview.eval("window.__TAURI_INTERNALS__.invoke('start_test')");
    }
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
        for scan in [0x1e, 0x63, 0x67, 0xe01d, 0xe11d] {
            assert_eq!(
                line(scan, false, 0x2a),
                "kt-input: key=other up=0 device=0x2a"
            );
        }
    }
}
