// The portable zip ships a marker file beside keytriage.exe. With the marker there, every app
// folder Tauri resolves, and so WebView2's profile, is one folder beside the exe, so deleting the
// unzipped folder removes everything the app wrote.
use std::fs::OpenOptions;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use windows::Win32::Storage::FileSystem::FILE_FLAG_DELETE_ON_CLOSE;

pub const MARKER: &str = "keytriage.portable";
pub const DATA: &str = "keytriage-data";

pub fn data_folder(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    dir.join(MARKER).is_file().then(|| dir.join(DATA))
}

// In a folder it can't write, such as Program Files, WebView2 fails with an error that names only an
// HRESULT, and the app must never move its data to %LOCALAPPDATA% without saying so. Windows
// deletes the probe when its handle closes, even if the app dies first.
pub fn prove_writable(data: &Path) -> io::Result<()> {
    std::fs::create_dir_all(data)?;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(FILE_FLAG_DELETE_ON_CLOSE.0)
        .open(data.join("write-check"))
        .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("keytriage-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_marker_beside_the_exe_puts_the_data_beside_it() {
        let dir = scratch("marker");
        let exe = dir.join("keytriage.exe");
        assert_eq!(data_folder(&exe), None);

        std::fs::create_dir(dir.join(MARKER)).unwrap();
        assert_eq!(data_folder(&exe), None, "a folder named like the marker");
        std::fs::remove_dir(dir.join(MARKER)).unwrap();

        std::fs::write(dir.join(MARKER), b"").unwrap();
        assert_eq!(data_folder(&exe), Some(dir.join("keytriage-data")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_writable_folder_passes_and_keeps_only_the_data_folder() {
        let dir = scratch("writable");
        let data = dir.join(DATA);
        prove_writable(&data).unwrap();
        prove_writable(&data).unwrap();
        assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_data_folder_that_cant_be_made_fails() {
        let dir = scratch("blocked");
        std::fs::write(dir.join("file"), b"").unwrap();
        assert!(prove_writable(&dir.join("file").join(DATA)).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
