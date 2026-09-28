// The Windows Save dialog for the report export: local COM, with no plugin. The chosen path stays
// here, and only the file's name goes back to the page.
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance,
    CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FileSaveDialog, IFileSaveDialog, SIGDN_FILESYSPATH,
};
use windows_core::{HSTRING, w};

// The dialog needs a single-threaded COM apartment, and a pooled thread may already hold COM in
// another mode, so it gets a thread of its own, as rfd gives it for tauri-plugin-dialog. The app
// window owns it from the window's own thread, whose messages keep flowing while it is open.
pub fn save(owner: isize, name: String, bytes: String) -> Result<Option<String>, String> {
    std::thread::spawn(move || save_on_this_thread(owner, &name, &bytes))
        .join()
        .map_err(|_| "The save dialog stopped unexpectedly.".to_string())?
}

struct Apartment;

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

fn save_on_this_thread(owner: isize, name: &str, bytes: &str) -> Result<Option<String>, String> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }
        .ok()
        .map_err(|e| format!("The save dialog couldn't open: {}", e.message()))?;
    // The dialog is released inside pick(), before the apartment closes.
    let _apartment = Apartment;
    let picked = unsafe { pick(owner, name) }
        .map_err(|e| format!("The save dialog couldn't open: {}", e.message()))?;
    let Some(path) = picked else {
        return Ok(None);
    };
    std::fs::write(&path, bytes).map_err(|e| format!("The report couldn't be saved: {e}"))?;
    let saved = path.file_name().unwrap_or_default();
    Ok(Some(saved.to_string_lossy().into_owned()))
}

// None when the user cancels.
unsafe fn pick(owner: isize, name: &str) -> windows_core::Result<Option<PathBuf>> {
    unsafe {
        let dialog: IFileSaveDialog =
            CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
        // A library or other shell location with no file path couldn't be written with std::fs.
        dialog.SetOptions(dialog.GetOptions()? | FOS_FORCEFILESYSTEM)?;
        dialog.SetFileTypes(&[COMDLG_FILTERSPEC {
            pszName: w!("JSON (*.json)"),
            pszSpec: w!("*.json"),
        }])?;
        dialog.SetDefaultExtension(w!("json"))?;
        dialog.SetFileName(&HSTRING::from(name))?;
        match dialog.Show(Some(HWND(owner as *mut _))) {
            Err(e) if e.code() == ERROR_CANCELLED.to_hresult() => return Ok(None),
            shown => shown?,
        }
        let chosen = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = PathBuf::from(OsString::from_wide(chosen.as_wide()));
        CoTaskMemFree(Some(chosen.0 as *const _));
        Ok(Some(path))
    }
}
