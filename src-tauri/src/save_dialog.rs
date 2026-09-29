// The Windows Save dialog for the report export: local COM, with no plugin. The chosen path stays
// here, and only the file's name goes back to the page.
use std::ffi::OsString;
use std::io;
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

use crate::view;

pub const STOPPED: &str = "The export stopped unexpectedly. Try Export report again.";
const NOT_OPENED: &str = "The Save dialog couldn't open. Try Export report again.";

// The dialog needs a single-threaded COM apartment, and a pooled thread may already hold COM in
// another mode, so it gets a thread of its own, as rfd gives it for tauri-plugin-dialog. The app
// window owns it from the window's own thread, whose messages keep flowing while it is open.
pub fn save(owner: isize, name: String, bytes: String) -> Result<Option<String>, String> {
    std::thread::spawn(move || save_on_this_thread(owner, &name, &bytes))
        .join()
        .map_err(|_| STOPPED.to_string())?
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
        .map_err(|e| view::failed(NOT_OPENED, e))?;
    // The dialog is released inside pick(), before the apartment closes.
    let _apartment = Apartment;
    let picked = unsafe { pick(owner, name) }.map_err(|e| view::failed(NOT_OPENED, e))?;
    let Some(path) = picked else {
        return Ok(None);
    };
    std::fs::write(&path, bytes).map_err(|e| save_error(&e))?;
    let saved = path.file_name().unwrap_or_default();
    Ok(Some(saved.to_string_lossy().into_owned()))
}

// The dialog lets the user pick a folder they can't write to, such as a protected one, or a full
// drive.
fn save_error(e: &io::Error) -> String {
    let sentence = match e.kind() {
        io::ErrorKind::PermissionDenied => {
            "Windows didn't allow saving there. Pick another folder, such as Documents, and try \
             again."
        }
        io::ErrorKind::StorageFull => "The drive is full. Pick another drive and try again.",
        _ => "The report couldn't be saved. Pick another folder and try again.",
    };
    view::failed(sentence, e)
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

#[cfg(test)]
mod tests {
    use super::*;

    // Windows words these messages in the user's language, so each check builds the detail from
    // the error itself.
    #[test]
    fn a_failed_save_says_what_to_try_first() {
        for (os_error, sentence) in [
            (
                5,
                "Windows didn't allow saving there. Pick another folder, such as Documents, and \
                 try again.",
            ),
            (112, "The drive is full. Pick another drive and try again."),
            (
                3,
                "The report couldn't be saved. Pick another folder and try again.",
            ),
        ] {
            let e = io::Error::from_raw_os_error(os_error);
            assert_eq!(save_error(&e), format!("{sentence} Details: {e}"));
        }
        assert_eq!(
            io::Error::from_raw_os_error(5).kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            io::Error::from_raw_os_error(112).kind(),
            io::ErrorKind::StorageFull
        );
    }
}
