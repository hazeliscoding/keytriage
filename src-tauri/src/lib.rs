use tauri::plugin::{Builder as PluginBuilder, TauriPlugin};
use tauri::{Manager, Runtime, Url};

#[cfg(windows)]
mod browser_ui;
#[cfg(windows)]
mod crash_reports;
#[cfg(all(debug_assertions, windows))]
mod echo;
#[cfg(windows)]
mod export;
#[cfg(all(test, windows))]
mod golden;
#[cfg(windows)]
mod save_dialog;
#[cfg(windows)]
mod session_core;
#[cfg(windows)]
mod test_session;
#[cfg(windows)]
mod view;

pub fn run() {
    #[cfg(windows)]
    if !positive_control("KEYTRIAGE_CRASH_REPORTS") {
        crash_reports::keep_app_crashes_local();
    }
    let builder = tauri::Builder::default()
        // tao registers Raw Input for every keyboard at startup unless this is Always, and Never
        // would add RIDEV_INPUTSINK. Capture belongs to crates/input, and only during a test.
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(navigation_guard())
        .setup(setup);
    #[cfg(windows)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        test_session::list_keyboards,
        test_session::start_test,
        test_session::start_swap_test,
        test_session::pause_test,
        test_session::continue_test,
        test_session::skip_key,
        test_session::end_test,
        test_session::export_report
    ]);
    #[cfg(all(debug_assertions, windows))]
    let builder = builder.on_page_load(echo::page_load);
    builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // A dump can come without a ProcessFailed event. WebView2 finishes shutting down after
            // the app exits, so a dump from a crash then waits for the next start.
            #[cfg(windows)]
            if let tauri::RunEvent::Exit = _event
                && let Some(reports) = _app.try_state::<ReportFolder>()
            {
                crash_reports::sweep(&reports.0);
            }
        });
}

#[cfg(windows)]
struct ReportFolder(std::path::PathBuf);

// The app checks' positive controls leave one protection off, in debug builds only.
#[cfg(windows)]
pub(crate) fn positive_control(name: &str) -> bool {
    cfg!(debug_assertions) && std::env::var_os(name).is_some()
}

// The window is built here rather than from the config, so that on Windows it gets a WebView2
// environment whose crash dumps stay on this machine.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config = app
        .config()
        .app
        .windows
        .first()
        .ok_or("tauri.conf.json defines no window")?
        .clone();
    let builder = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?;
    #[cfg(windows)]
    let (builder, reports) = if positive_control("KEYTRIAGE_CRASH_REPORTS") {
        (builder, None)
    } else {
        let data_dir = app.path().app_local_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;
        let environment = crash_reports::environment(&data_dir)?;
        let reports = crash_reports::report_folder(&environment)?;
        crash_reports::sweep(&reports);
        app.manage(ReportFolder(reports.clone()));
        (builder.with_environment(environment), Some(reports))
    };
    let window = builder.build()?;
    #[cfg(windows)]
    {
        if let Some(reports) = reports {
            crash_reports::sweep_on_failure(&window, reports)?;
        }
        if !positive_control("KEYTRIAGE_BROWSER_KEYS") {
            browser_ui::turn_off(&window)?;
        }
        test_session::end_with_page(&window)?;
        #[cfg(debug_assertions)]
        echo::start(&window)?;
    }
    #[cfg(not(windows))]
    let _ = window;
    Ok(())
}

// The CSP can't stop the page from navigating itself, so a link, a location change or a meta
// refresh could load a remote page in the app window.
fn navigation_guard<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("navigation-guard")
        .on_navigation(|webview, url| {
            // A reload or history move during a test would drop the page and its view of the
            // test. The browser keys setting stops the keyboard's own routes, but a page script, a
            // mouse side button or a runtime that ignores the setting can still navigate. A refused
            // reload keeps the page and the test.
            #[cfg(windows)]
            if test_session::running() && !positive_control("KEYTRIAGE_RELOADS") {
                #[cfg(debug_assertions)]
                echo::note("kt-shell: navigation-refused");
                return false;
            }
            let dev_url = if cfg!(dev) {
                webview.config().build.dev_url.as_ref()
            } else {
                None
            };
            is_app_url(url, dev_url)
        })
        .build()
}

fn is_app_url(url: &Url, dev_url: Option<&Url>) -> bool {
    // WebView2 serves the bundled frontend from this origin.
    url.origin().ascii_serialization() == "http://tauri.localhost"
        || dev_url.is_some_and(|dev| dev.origin() == url.origin())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn allows_the_bundled_app() {
        assert!(is_app_url(&url("http://tauri.localhost/"), None));
        assert!(is_app_url(
            &url("http://tauri.localhost:80/index.html#top"),
            None
        ));
    }

    #[test]
    fn allows_the_dev_server_only_in_dev() {
        let dev = url("http://localhost:4200");
        assert!(is_app_url(&url("http://localhost:4200/"), Some(&dev)));
        assert!(!is_app_url(&url("http://localhost:4200/"), None));
    }

    #[test]
    fn denies_everything_else() {
        let dev = url("http://localhost:4200");
        for s in [
            "https://tauri.localhost/",
            "http://tauri.localhost.example.com/",
            "https://example.com/",
            "http://localhost:4201/",
            "http://127.0.0.1:4200/",
            "about:blank",
            "data:text/html,x",
            "file:///C:/x.html",
        ] {
            assert!(!is_app_url(&url(s), Some(&dev)), "{s}");
        }
    }
}
