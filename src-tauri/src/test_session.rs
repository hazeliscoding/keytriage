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
    // Keys still down when the app lost the foreground never report their release, so they are
    // listed as interrupted rather than left to look stuck.
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
    SESSION.with_borrow_mut(|session| {
        *session = Some(Session {
            _capture: capture,
            _entries: entries,
        })
    });
    RUNNING.store(true, Ordering::SeqCst);
    let _ = window.emit("test:started", ());
    Ok(())
}

#[tauri::command]
pub fn stop_test() {
    SESSION.with_borrow_mut(|session| *session = None);
    RUNNING.store(false, Ordering::SeqCst);
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
