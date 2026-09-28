// A test's events live in memory only: here, in order, and in the page, which gets each one as it
// happens. Nothing about their order is written anywhere.
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use keytriage_input::{Capture, Input};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
};
use webview2_com::ProcessFailedEventHandler;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Entry {
    Key {
        scan: u16,
        up: bool,
        device: isize,
        // Microseconds since the test started, here and below.
        micros: u64,
    },
    // A key still down when the app lost the foreground may be released while it is away, which
    // capture never sees, so it is listed as interrupted rather than left to look stuck. If it is
    // still held at the resume, its release, and any repeats, arrive after Resumed.
    Paused {
        micros: u64,
        interrupted: Vec<HeldKey>,
    },
    Resumed {
        micros: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HeldKey {
    pub device: isize,
    pub scan: u16,
}

struct Recorder {
    start: Instant,
    held: BTreeSet<HeldKey>,
}

impl Recorder {
    fn micros(&self, at: Instant) -> u64 {
        at.saturating_duration_since(self.start).as_micros() as u64
    }

    fn entry(&mut self, input: Input) -> Entry {
        match input {
            Input::Key(event) => {
                let key = HeldKey {
                    device: event.device,
                    scan: event.scan,
                };
                if event.up {
                    self.held.remove(&key);
                } else {
                    self.held.insert(key);
                }
                Entry::Key {
                    scan: event.scan,
                    up: event.up,
                    device: event.device,
                    micros: self.micros(event.at),
                }
            }
            Input::Paused(at) => Entry::Paused {
                micros: self.micros(at),
                interrupted: std::mem::take(&mut self.held).into_iter().collect(),
            },
            Input::Resumed(at) => Entry::Resumed {
                micros: self.micros(at),
            },
        }
    }
}

struct Session {
    _capture: Capture,
    _entries: Rc<RefCell<Vec<Entry>>>,
    page: WebviewWindow,
}

// Capture lives on the window's thread, which is also where Tauri runs synchronous commands.
thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

// The navigation guard runs on the same thread, but reads this without borrowing the session.
static RUNNING: AtomicBool = AtomicBool::new(false);

pub fn running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn start_test(window: WebviewWindow) -> Result<(), String> {
    let started = start(window);
    // The reason is a Win32 or Tauri error, never key data.
    #[cfg(debug_assertions)]
    if let Err(e) = &started {
        crate::echo::note(&format!("kt-input: start failed: {e}"));
    }
    started
}

fn start(window: WebviewWindow) -> Result<(), String> {
    stop_test();
    let mut recorder = Recorder {
        start: Instant::now(),
        held: BTreeSet::new(),
    };
    let entries = Rc::new(RefCell::new(Vec::new()));
    let (buffer, page) = (entries.clone(), window.clone());
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let capture = Capture::start(hwnd.0 as isize, move |input| {
        let entry = recorder.entry(input);
        let _ = page.emit("test:event", &entry);
        buffer.borrow_mut().push(entry);
    })
    .map_err(|e| e.to_string())?;
    let open = capture.is_open();
    SESSION.with_borrow_mut(|session| {
        *session = Some(Session {
            _capture: capture,
            _entries: entries.clone(),
            page: window.clone(),
        })
    });
    RUNNING.store(true, Ordering::SeqCst);
    let _ = window.emit("test:started", ());
    // A test started while the app is in the background begins paused, and says so, so that every
    // Resumed follows a Paused. Nothing pumps messages between here and Capture::start.
    if !open {
        let entry = Entry::Paused {
            micros: 0,
            interrupted: Vec::new(),
        };
        let _ = window.emit("test:event", &entry);
        entries.borrow_mut().push(entry);
    }
    Ok(())
}

#[tauri::command]
pub fn stop_test() {
    if let Some(session) = SESSION.with_borrow_mut(Option::take) {
        RUNNING.store(false, Ordering::SeqCst);
        let _ = session.page.emit("test:stopped", ());
    }
}

// A test belongs to the page that started it. If the page's process dies, the test ends, so that
// capture stops and the refused reloads no longer keep the window on an error page.
pub fn end_with_page(window: &WebviewWindow) -> tauri::Result<()> {
    window.with_webview(|webview| {
        let handler = ProcessFailedEventHandler::create(Box::new(|sender, args| {
            let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
            if let Some(args) = args {
                unsafe { args.ProcessFailedKind(&mut kind)? };
            }
            if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
                || kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED
            {
                stop_test();
                if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
                    && let Some(core) = sender
                {
                    unsafe { core.Reload()? };
                }
            }
            Ok(())
        }));
        let mut token = 0;
        let watched = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|core| core.add_ProcessFailed(&handler, &mut token))
        };
        if let Err(e) = watched {
            eprintln!("keytriage: could not watch the page for a crash: {e}");
        }
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use keytriage_input::KeyEvent;

    use super::*;

    fn recorder() -> Recorder {
        Recorder {
            start: Instant::now(),
            held: BTreeSet::new(),
        }
    }

    fn key(recorder: &Recorder, scan: u16, up: bool, micros: u64) -> Input {
        Input::Key(KeyEvent {
            scan,
            up,
            device: 7,
            at: recorder.start + Duration::from_micros(micros),
        })
    }

    #[test]
    fn records_time_since_the_test_started() {
        let mut r = recorder();
        let input = key(&r, 0x1e, true, 1500);
        assert_eq!(
            r.entry(input),
            Entry::Key {
                scan: 0x1e,
                up: true,
                device: 7,
                micros: 1500
            }
        );
        let early = Input::Resumed(r.start - Duration::from_millis(1));
        assert_eq!(r.entry(early), Entry::Resumed { micros: 0 });
    }

    #[test]
    fn a_pause_lists_the_keys_still_down_as_interrupted() {
        let mut r = recorder();
        for input in [
            key(&r, 0x1e, false, 10),
            key(&r, 0x1f, false, 20),
            key(&r, 0x1f, false, 30),
            key(&r, 0x1e, true, 40),
        ] {
            r.entry(input);
        }
        let paused = Input::Paused(r.start + Duration::from_micros(50));
        assert_eq!(
            r.entry(paused),
            Entry::Paused {
                micros: 50,
                interrupted: vec![HeldKey {
                    device: 7,
                    scan: 0x1f
                }]
            }
        );
        let again = Input::Paused(r.start + Duration::from_micros(60));
        assert_eq!(
            r.entry(again),
            Entry::Paused {
                micros: 60,
                interrupted: vec![]
            }
        );
    }
}
