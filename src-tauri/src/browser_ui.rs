// WebView2 reloads the page on F5 and Ctrl+R and prints it on Ctrl+P, and its context menu offers
// reload and inspect. A key test presses all of those. Tauri has no setting for either, so they
// are turned off on the live webview.
use tauri::{Runtime, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2Settings3, ICoreWebView2Settings8,
};
use windows_core::Interface;

pub fn turn_off<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.with_webview(|webview| {
        if let Err(e) = apply(&webview.controller()) {
            eprintln!("keytriage: could not change WebView2's settings: {e}");
        }
    })
}

fn apply(controller: &ICoreWebView2Controller) -> windows_core::Result<()> {
    unsafe {
        let settings = controller.CoreWebView2()?.Settings()?;
        settings.SetAreDefaultContextMenusEnabled(false)?;
        settings
            .cast::<ICoreWebView2Settings3>()?
            .SetAreBrowserAcceleratorKeysEnabled(false)?;
        // SmartScreen asks Microsoft about the addresses the page loads. The
        // msSmartScreenProtection flag in crash_reports.rs turns it off too, but Microsoft says
        // production apps shouldn't rely on browser flags, and this is its supported switch. It
        // goes last, so a runtime without Settings8 still gets the other two.
        settings
            .cast::<ICoreWebView2Settings8>()?
            .SetIsReputationCheckingRequired(false)
    }
}
