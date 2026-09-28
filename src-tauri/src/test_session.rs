// A test's events live in memory only: here, in order, and in the page, which gets each one as it
// happens. Nothing about their order is written anywhere.
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use keytriage_input::{Capture, Input};
use tauri::{Emitter, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
};
use webview2_com::ProcessFailedEventHandler;

use crate::session_core::{Core, Entry};
use crate::view::{self, GuideView, KeyName, KeyboardGroup, PlanArgs, TestResult, groups};
use crate::{export, save_dialog};

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

// The last ended test's export, until the next test starts. The ordered events are gone by then,
// and the export command reads this from another thread.
static REPORT: Mutex<Option<String>> = Mutex::new(None);

const NO_TEST: &str = "No test is running.";
const NO_PLAN: &str = "This test has no plan to diagnose.";

fn set_report(report: Option<String>) {
    *REPORT.lock().unwrap_or_else(PoisonError::into_inner) = report;
}

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
    if let Some((_, keyboard, _)) = &test {
        let listed = keytriage_input::keyboards().map_err(|e| e.to_string())?;
        view::still_listed(keyboard, &listed)?;
    }
    stop_test();
    let start = Instant::now();
    let core = Rc::new(RefCell::new(Core::new(start, test)));
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let capture = capture(hwnd, &core, &window)?;
    // A failed Test again leaves the last findings on the page, so their report stays exportable
    // until a new test has actually started.
    set_report(None);
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
// The page names the step it shows, and a skip for any other step does nothing.
#[tauri::command]
pub fn skip_key(round: u16, index: u16) -> Result<(), String> {
    let (core, page, user_paused) = SESSION
        .with_borrow(|session| {
            let s = session.as_ref()?;
            Some((s.core.clone(), s.page.clone(), s.user_paused))
        })
        .ok_or(NO_TEST)?;
    let skipped = core.borrow_mut().skip(Instant::now(), round, index);
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

// End test diagnoses what the test saw. The names only word the findings for the page. A refusal
// leaves the test running.
#[tauri::command]
pub fn end_test(labels: Vec<KeyName>) -> Result<TestResult, String> {
    let names = view::names(labels)?;
    let session = SESSION.with_borrow_mut(|session| match session {
        Some(s) if !s.core.borrow().guided() => Err(NO_PLAN),
        _ => session.take().ok_or(NO_TEST),
    })?;
    RUNNING.store(false, Ordering::SeqCst);
    let Session {
        capture,
        core,
        page,
        ..
    } = session;
    // Once capture is gone no input can reach Core, so the diagnosis sees the whole test.
    drop(capture);
    let report = core.borrow_mut().finish(Instant::now()).ok_or(NO_PLAN)?;
    set_report(Some(export::report_json(&report.saved())));
    let result = view::result(&report, &names);
    let _ = page.emit("test:stopped", ());
    Ok(result)
}

// The only way a test's data reaches the disk. A sync command would run the dialog's modal loop
// inside the window thread's IPC callback, so this one is async, and it reads only REPORT and
// RUNNING, because SESSION lives on the window's thread. It returns the saved file's name, or None
// when the user cancels.
#[tauri::command]
pub async fn export_report(window: WebviewWindow, name: String) -> Result<Option<String>, String> {
    export::check_file_name(&name)?;
    if running() {
        return Err("A report can't be exported while a test runs.".to_string());
    }
    let bytes = REPORT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .ok_or("There is no report to export.")?;
    let owner = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    tauri::async_runtime::spawn_blocking(move || save_dialog::save(owner, name, bytes))
        .await
        .map_err(|e| e.to_string())?
}

fn stop_test() {
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
