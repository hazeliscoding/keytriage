// Debug builds only. With KEYTRIAGE_ECHO set, capture starts with the window and prints each
// event, and page loads and WebView2's browser settings print too, so the checks in scripts/ can
// prove where input stops and that browser keys do nothing. Only the checks' marker keys, F13 to
// F15, print their scan code, so real typing never shows up in a terminal or a CI log.
use std::io::Write;

use keytriage_input::{Capture, KeyEvent, registrations};
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{Runtime, Webview, WebviewWindow};
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
    let capture = Capture::start(window.hwnd()?.0 as isize, |event| print(&line(&event)))?;
    for r in registrations()? {
        print(&format!(
            "kt-input: registered page=0x{:x} usage=0x{:x} flags=0x{:x} target=0x{:x}",
            r.usage_page, r.usage, r.flags, r.target
        ));
    }
    print(&format!("kt-shell: error-mode=0x{:x}", unsafe {
        GetErrorMode()
    }));
    print("kt-input: ready");
    // with_webview calls run in order, so this reads the settings after browser_ui changed them.
    window.with_webview(|webview| print(&settings_line(&webview.controller())))?;
    // The echo lasts as long as the window.
    std::mem::forget(capture);
    Ok(())
}

pub fn page_load<R: Runtime>(_: &Webview<R>, payload: &PageLoadPayload<'_>) {
    if enabled() && payload.event() == PageLoadEvent::Finished {
        print("kt-shell: page-load");
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

fn line(event: &KeyEvent) -> String {
    let key = match event.scan {
        0x64..=0x66 => format!("0x{:x}", event.scan),
        _ => "other".to_string(),
    };
    format!(
        "kt-input: key={key} up={} device=0x{:x}",
        u8::from(event.up),
        event.device
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(scan: u16, up: bool) -> KeyEvent {
        KeyEvent {
            scan,
            up,
            device: 0x2a,
        }
    }

    #[test]
    fn prints_the_marker_keys() {
        assert_eq!(
            line(&event(0x64, false)),
            "kt-input: key=0x64 up=0 device=0x2a"
        );
        assert_eq!(
            line(&event(0x66, true)),
            "kt-input: key=0x66 up=1 device=0x2a"
        );
    }

    #[test]
    fn hides_every_other_key() {
        for scan in [0x1e, 0x63, 0x67, 0xe01d, 0xe11d] {
            assert_eq!(
                line(&event(scan, false)),
                "kt-input: key=other up=0 device=0x2a"
            );
        }
    }
}
