use tauri::plugin::{Builder as PluginBuilder, TauriPlugin};
use tauri::{Manager, Runtime, Url};

#[cfg(windows)]
mod browser_ui;
#[cfg(all(debug_assertions, windows))]
mod echo;

pub fn run() {
    let builder = tauri::Builder::default()
        // tao registers Raw Input for every keyboard at startup unless this is Always, and Never
        // would add RIDEV_INPUTSINK. Capture belongs to crates/input, and only during a test.
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(navigation_guard());
    #[cfg(windows)]
    let builder = builder.setup(setup);
    #[cfg(all(debug_assertions, windows))]
    let builder = builder.on_page_load(echo::page_load);
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(windows)]
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("the main window is missing")?;
    // The browser keys check's positive control leaves them on, in debug builds only.
    let keep_browser_keys =
        cfg!(debug_assertions) && std::env::var_os("KEYTRIAGE_BROWSER_KEYS").is_some();
    if !keep_browser_keys {
        browser_ui::turn_off(&window)?;
    }
    #[cfg(debug_assertions)]
    echo::start(&window)?;
    Ok(())
}

// The CSP can't stop the page from navigating itself, so a link, a location change or a meta
// refresh could load a remote page in the app window.
fn navigation_guard<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("navigation-guard")
        .on_navigation(|webview, url| {
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
