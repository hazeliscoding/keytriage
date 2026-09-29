// What a failed start tells the user. A message box needs neither the app's window nor WebView2,
// either of which may be what failed.
use std::fmt::Display;

use webview2_com::Microsoft::Web::WebView2::Win32::GetAvailableCoreWebView2BrowserVersionString;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows_core::{HSTRING, PCWSTR, PWSTR, w};

pub const NO_RUNTIME: &str = "keytriage needs Microsoft's WebView2 Runtime, and it isn't \
                              installed. Install the Evergreen WebView2 Runtime from \
                              developer.microsoft.com/microsoft-edge/webview2, then start \
                              keytriage again.";

pub fn failed(detail: impl Display) -> String {
    format!(
        "keytriage couldn't start. Start it again. If it keeps failing, report it in the \
         project's GitHub issues with this text. Details: {detail}"
    )
}

// Without a runtime the window's environment fails with an error that names only an HRESULT. The
// loader honours WEBVIEW2_BROWSER_EXECUTABLE_FOLDER, which is how the installer check fakes a
// missing runtime.
pub fn runtime_missing() -> bool {
    let mut version = PWSTR::null();
    let found =
        unsafe { GetAvailableCoreWebView2BrowserVersionString(PCWSTR::null(), &mut version) };
    let version = webview2_com::take_pwstr(version);
    found.is_err() || version.is_empty()
}

pub fn show(text: &str) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            w!("keytriage"),
            MB_OK | MB_ICONERROR,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_runtime_is_named_with_the_next_step() {
        assert!(NO_RUNTIME.starts_with("keytriage needs Microsoft's WebView2 Runtime, and it"));
        assert!(NO_RUNTIME.contains(" isn't installed. Install the Evergreen WebView2 Runtime "));
        assert!(NO_RUNTIME.ends_with(", then start keytriage again."));
        assert!(!NO_RUNTIME.contains("  "));
    }

    #[test]
    fn a_failed_start_leads_with_what_to_try_and_ends_with_the_detail() {
        assert_eq!(
            failed("error encountered during setup hook: Access is denied. (0x80070005)"),
            "keytriage couldn't start. Start it again. If it keeps failing, report it in the \
             project's GitHub issues with this text. Details: error encountered during setup \
             hook: Access is denied. (0x80070005)"
        );
    }
}
