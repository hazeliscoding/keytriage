// A test's events live in memory only: here, in order, and in the page, which gets each one as it
// happens. Nothing about their order is written anywhere.
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use keytriage_input::{Capture, Input};
use tauri::{Emitter, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
};
use webview2_com::ProcessFailedEventHandler;

use crate::session_core::{Core, Entry};
use crate::view::{self, GuideView, KeyboardGroup, PlanArgs, groups};

// The capture callback and the commands share Core on this one thread. None of them holds a borrow
// of it, or of SESSION, across Capture::start, a Capture drop or an emit.
struct Session {
    // None while the user has paused the test, so nothing is registered then.
    capture: Option<Capture>,
    core: Rc<RefCell<Core>>,
    page: WebviewWindow,
    hwnd: isize,
    user_paused: bool,
}

// Capture lives on the window's thread, which is also where Tauri runs synchronous commands.
thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

// The navigation guard runs on the same thread, but reads this without borrowing the session.
static RUNNING: AtomicBool = AtomicBool::new(false);

const NO_TEST: &str = "No test is running.";

pub fn running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

// Listing reads no keys, so it needs no test running.
#[tauri::command]
pub fn list_keyboards() -> Result<Vec<KeyboardGroup>, String> {
    let keyboards = keytriage_input::keyboards().map_err(|e| e.to_string())?;
    Ok(groups(&keyboards))
}

// Without a plan, as the debug echo starts it, the test only records.
#[tauri::command]
pub fn start_test(window: WebviewWindow, plan: Option<PlanArgs>) -> Result<(), String> {
    let started = start(window, plan);
    // The reason is a Win32 or Tauri error, or a refused plan, never key data.
    #[cfg(debug_assertions)]
    if let Err(e) = &started {
        crate::echo::note(&format!("kt-input: start failed: {e}"));
    }
    started
}

fn start(window: WebviewWindow, plan: Option<PlanArgs>) -> Result<(), String> {
    let test = plan.map(view::plan).transpose()?;
    stop_test();
    let start = Instant::now();
    let core = Rc::new(RefCell::new(Core::new(start, test)));
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let capture = capture(hwnd, &core, &window)?;
    let open = capture.is_open();
    SESSION.with_borrow_mut(|session| {
        *session = Some(Session {
            capture: Some(capture),
            core: core.clone(),
            page: window.clone(),
            hwnd,
            user_paused: false,
        })
    });
    RUNNING.store(true, Ordering::SeqCst);
    let _ = window.emit("test:started", ());
    let first = core.borrow().view();
    if let Some(view) = first {
        let _ = window.emit("test:guide", &view);
    }
    // A test started while the app is in the background begins paused, and says so, so that every
    // Resumed follows a Paused. Nothing pumps messages between here and Capture::start.
    if !open {
        record(&core, &window, Input::Paused(start));
    }
    Ok(())
}

fn capture(hwnd: isize, core: &Rc<RefCell<Core>>, page: &WebviewWindow) -> Result<Capture, String> {
    let (core, page) = (core.clone(), page.clone());
    Capture::start(hwnd, move |input| record(&core, &page, input)).map_err(|e| e.to_string())
}

// The window procedure runs this for capture. A borrow already out would mean a command pumped
// messages while holding Core, and the input is dropped rather than panicking in the window
// procedure.
fn record(core: &RefCell<Core>, page: &WebviewWindow, input: Input) {
    let Ok(mut core) = core.try_borrow_mut() else {
        return;
    };
    let (entry, view) = core.input(input);
    drop(core);
    emit(page, &entry, view);
}

fn emit(page: &WebviewWindow, entry: &Entry, view: Option<GuideView>) {
    let _ = page.emit("test:event", entry);
    if let Some(view) = view {
        let _ = page.emit("test:guide", &view);
    }
}

// The Pause button stops capture, as a focus loss does, so no key is read until Continue. The
// Capture is taken out of the session and dropped, which unregisters Raw Input and removes the
// subclass, and only then is the pause recorded.
#[tauri::command]
pub fn pause_test() -> Result<(), String> {
    let taken = SESSION
        .with_borrow_mut(|session| {
            let s = session.as_mut()?;
            let first = !std::mem::replace(&mut s.user_paused, true);
            Some(first.then(|| {
                let open = s.capture.as_ref().is_some_and(Capture::is_open);
                // The focus check's positive control for a user pause: capture keeps reading.
                let capture = if crate::positive_control("KEYTRIAGE_USER_PAUSE_CAPTURES") {
                    None
                } else {
                    s.capture.take()
                };
                (capture, open, s.core.clone(), s.page.clone())
            }))
        })
        .ok_or(NO_TEST)?;
    let Some((capture, open, core, page)) = taken else {
        return Ok(());
    };
    drop(capture);
    // A capture that a focus loss had closed has already recorded its own pause.
    if open {
        record(&core, &page, Input::Paused(Instant::now()));
    }
    Ok(())
}

#[tauri::command]
pub fn continue_test() -> Result<(), String> {
    let found = SESSION
        .with_borrow_mut(|session| {
            let s = session.as_mut()?;
            Some(
                s.user_paused
                    .then(|| (s.capture.take(), s.core.clone(), s.page.clone(), s.hwnd)),
            )
        })
        .ok_or(NO_TEST)?;
    let Some((kept, core, page, hwnd)) = found else {
        return Ok(());
    };
    // Only the user pause positive control leaves a Capture here. Dropping one unregisters Raw
    // Input for the whole process, so it goes before the new Capture registers.
    drop(kept);
    let capture = capture(hwnd, &core, &page)?;
    let open = capture.is_open();
    let unused = SESSION.with_borrow_mut(|session| match session {
        Some(s) if Rc::ptr_eq(&s.core, &core) => {
            s.user_paused = false;
            s.capture.replace(capture)
        }
        _ => Some(capture),
    });
    drop(unused);
    // In the background capture stays closed, and records the resume when the app comes back.
    if open {
        record(&core, &page, Input::Resumed(Instant::now()));
    }
    Ok(())
}

// Skip closes the round as asked, so a key that never registers leaves silent rounds. During a
// user pause there is no open round, so it only moves on, and the test continues with the next key.
#[tauri::command]
pub fn skip_key() -> Result<(), String> {
    let (core, page, user_paused) = SESSION
        .with_borrow(|session| {
            let s = session.as_ref()?;
            Some((s.core.clone(), s.page.clone(), s.user_paused))
        })
        .ok_or(NO_TEST)?;
    let skipped = core.borrow_mut().skip(Instant::now());
    let Some(view) = skipped else {
        return Ok(());
    };
    let more = view.key.is_some();
    let _ = page.emit("test:guide", &view);
    if user_paused && more {
        continue_test()?;
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
