// A crash dump can hold the ordered key events a process keeps in memory, so no dump may leave the
// machine or stay on disk. WebView2 uploads its dumps to Microsoft unless the app creates the
// environment with custom crash reporting, and even then it writes them under the user data
// folder, so they are deleted as they appear. The app's own crashes never reach Windows Error
// Reporting.
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use tauri::{Runtime, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    CreateCoreWebView2EnvironmentWithOptions, ICoreWebView2Environment, ICoreWebView2Environment11,
    ICoreWebView2EnvironmentOptions,
};
use webview2_com::{
    CoreWebView2EnvironmentOptions, CreateCoreWebView2EnvironmentCompletedHandler,
    ProcessFailedEventHandler,
};
use windows::Win32::Foundation::{E_POINTER, E_UNEXPECTED};
use windows::Win32::Globalization::{
    GetUserDefaultUILanguage, LCIDToLocaleName, LOCALE_ALLOW_NEUTRAL_NAMES, MAX_LOCALE_NAME,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::Diagnostics::Debug::{
    GetErrorMode, SEM_NOGPFAULTERRORBOX, SetErrorMode, THREAD_ERROR_MODE,
};
use windows_core::{HSTRING, Interface, PCWSTR, PWSTR};

// This covers Rust aborts too, which skip every in-process handler. WebView2's browser process sets
// its own mode, and its renderers already run without Windows Error Reporting.
pub fn keep_app_crashes_local() {
    unsafe { SetErrorMode(THREAD_ERROR_MODE(GetErrorMode() | SEM_NOGPFAULTERRORBOX.0)) };
}

// wry 0.57 sets these same options, with Tauri's defaults, when it makes the environment itself, but
// it can't turn on custom crash reporting.
pub fn environment(data_dir: &Path) -> windows_core::Result<ICoreWebView2Environment> {
    let options = CoreWebView2EnvironmentOptions::default();
    let (tx, rx) = mpsc::channel();
    unsafe {
        // wry would do this before its first webview, and tao's windows need the same apartment.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        options.set_additional_browser_arguments(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
             --autoplay-policy=no-user-gesture-required"
                .to_string(),
        );
        options.set_are_browser_extensions_enabled(false);
        options.set_language(user_language());
        options.set_is_custom_crash_reporting_enabled(true);
        CreateCoreWebView2EnvironmentWithOptions(
            PCWSTR::null(),
            &HSTRING::from(data_dir.as_os_str()),
            &ICoreWebView2EnvironmentOptions::from(options),
            &CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
                move |created, environment| {
                    let result = created.and_then(|()| environment.ok_or_else(|| E_POINTER.into()));
                    tx.send(result).map_err(|_| E_UNEXPECTED.into())
                },
            )),
        )?;
    }
    webview2_com::wait_with_pump(rx).map_err(|_| windows_core::Error::from(E_UNEXPECTED))?
}

fn user_language() -> String {
    let mut name = [0u16; MAX_LOCALE_NAME as usize];
    unsafe {
        LCIDToLocaleName(
            u32::from(GetUserDefaultUILanguage()),
            Some(&mut name),
            LOCALE_ALLOW_NEUTRAL_NAMES,
        )
    };
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    String::from_utf16_lossy(&name[..end])
}

// Crashpad writes each dump into this folder, under the user data folder.
pub fn report_folder(environment: &ICoreWebView2Environment) -> windows_core::Result<PathBuf> {
    let mut path = PWSTR::null();
    unsafe {
        environment
            .cast::<ICoreWebView2Environment11>()?
            .FailureReportFolderPath(&mut path)?
    };
    Ok(PathBuf::from(webview2_com::take_pwstr(path)))
}

// Crashpad's other files hold IDs and the app's name, not process memory.
pub fn sweep(reports: &Path) {
    remove_contents(reports);
    if let Some(crashpad) = reports.parent() {
        remove_contents(&crashpad.join("attachments"));
    }
}

fn remove_contents(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        let _ = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
    }
}

// The dump is complete by the time WebView2 reports the failure.
pub fn sweep_on_failure<R: Runtime>(
    window: &WebviewWindow<R>,
    reports: PathBuf,
) -> tauri::Result<()> {
    window.with_webview(move |webview| {
        let handler = ProcessFailedEventHandler::create(Box::new(move |_, _| {
            sweep(&reports);
            Ok(())
        }));
        let mut token = 0;
        let watched = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|core| core.add_ProcessFailed(&handler, &mut token))
        };
        match watched {
            #[cfg(debug_assertions)]
            Ok(()) => crate::echo::note("kt-shell: crash-watch=ok"),
            #[cfg(not(debug_assertions))]
            Ok(()) => {}
            Err(e) => eprintln!("keytriage: could not watch WebView2 for crashes: {e}"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweeps_dumps_and_attachments_but_keeps_the_folders() {
        let root = std::env::temp_dir().join(format!("keytriage-sweep-{}", std::process::id()));
        let reports = root.join("reports");
        let attachment = root.join("attachments").join("one");
        std::fs::create_dir_all(&reports).unwrap();
        std::fs::create_dir_all(&attachment).unwrap();
        std::fs::write(reports.join("a.dmp"), b"MDMP").unwrap();
        std::fs::write(attachment.join("log.txt"), b"x").unwrap();
        std::fs::write(root.join("settings.dat"), b"ids").unwrap();

        sweep(&reports);

        assert_eq!(std::fs::read_dir(&reports).unwrap().count(), 0);
        assert_eq!(
            std::fs::read_dir(root.join("attachments")).unwrap().count(),
            0
        );
        assert!(root.join("settings.dat").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
