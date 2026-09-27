// Debug builds only. With KEYTRIAGE_ECHO_INPUT set, capture starts with the window and prints each
// event, so scripts/check-focus-capture.ps1 can prove where input stops. Only that check's marker
// keys, F13 to F15, print their scan code, so real typing never shows up in a terminal or a CI log.
use std::io::Write;

use keytriage_input::{Capture, KeyEvent, registrations};
use tauri::{App, Manager};

pub fn start(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("KEYTRIAGE_ECHO_INPUT").is_none() {
        return Ok(());
    }
    let window = app
        .get_webview_window("main")
        .ok_or("the main window is missing")?;
    let capture = Capture::start(window.hwnd()?.0 as isize, |event| print(&line(&event)))?;
    for r in registrations()? {
        print(&format!(
            "kt-input: registered page=0x{:x} usage=0x{:x} flags=0x{:x} target=0x{:x}",
            r.usage_page, r.usage, r.flags, r.target
        ));
    }
    print("kt-input: ready");
    // The echo lasts as long as the window.
    std::mem::forget(capture);
    Ok(())
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
