// A test's events live in memory only: here, in order, and in the page, which gets each one as it
// happens. Nothing about their order is written anywhere.
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use keytriage_diagnostics as diagnostics;
use keytriage_input::Capture;
use tauri::{Emitter, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
};
use webview2_com::ProcessFailedEventHandler;

use crate::session_core::{Entry, Recorder, to_engine};

struct Session {
    _capture: Capture,
    _entries: Rc<RefCell<Vec<diagnostics::Entry>>>,
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
    let mut recorder = Recorder::new(Instant::now());
    let entries = Rc::new(RefCell::new(Vec::new()));
    let (buffer, page) = (entries.clone(), window.clone());
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let capture = Capture::start(hwnd.0 as isize, move |input| {
        let entry = recorder.entry(input);
        let _ = page.emit("test:event", &entry);
        buffer.borrow_mut().push(to_engine(&entry));
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
        entries.borrow_mut().push(to_engine(&entry));
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
