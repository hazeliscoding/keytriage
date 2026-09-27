pub fn run() {
    tauri::Builder::default()
        // tao registers Raw Input for every keyboard at startup unless this is Always, and Never
        // would add RIDEV_INPUTSINK. Capture belongs to crates/input, and only during a test.
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
