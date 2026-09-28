// A test's key events live in memory only: here, in order, and in the page, which gets each one as
// it happens. Nothing about their order is written anywhere.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use keytriage_input::{Capture, KeyEvent};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, WebviewWindow};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recorded {
    pub scan: u16,
    pub up: bool,
    pub device: isize,
    // Microseconds since the test started.
    pub micros: u64,
}

impl Recorded {
    fn new(event: &KeyEvent, start: Instant) -> Self {
        Recorded {
            scan: event.scan,
            up: event.up,
            device: event.device,
            micros: event.at.saturating_duration_since(start).as_micros() as u64,
        }
    }
}

struct Session {
    _capture: Capture,
    _events: Rc<RefCell<Vec<Recorded>>>,
}

// Capture lives on the window's thread, which is also where Tauri runs synchronous commands.
thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

#[tauri::command]
pub fn start_test(window: WebviewWindow) -> Result<(), String> {
    stop_test();
    let start = Instant::now();
    let events = Rc::new(RefCell::new(Vec::new()));
    let (buffer, page) = (events.clone(), window.clone());
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let capture = Capture::start(hwnd.0 as isize, move |event| {
        let recorded = Recorded::new(&event, start);
        let _ = page.emit("test:key", &recorded);
        buffer.borrow_mut().push(recorded);
    })
    .map_err(|e| e.to_string())?;
    SESSION.with_borrow_mut(|session| {
        *session = Some(Session {
            _capture: capture,
            _events: events,
        })
    });
    let _ = window.emit("test:started", ());
    Ok(())
}

#[tauri::command]
pub fn stop_test() {
    SESSION.with_borrow_mut(|session| *session = None);
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn records_time_since_the_test_started() {
        let start = Instant::now();
        let event = KeyEvent {
            scan: 0x1e,
            up: true,
            device: 7,
            at: start + Duration::from_micros(1500),
        };
        assert_eq!(
            Recorded::new(&event, start),
            Recorded {
                scan: 0x1e,
                up: true,
                device: 7,
                micros: 1500
            }
        );
    }

    #[test]
    fn an_event_read_before_the_start_counts_as_zero() {
        let at = Instant::now();
        let event = KeyEvent {
            scan: 0x1e,
            up: false,
            device: 7,
            at,
        };
        assert_eq!(
            Recorded::new(&event, at + Duration::from_millis(1)).micros,
            0
        );
    }
}
