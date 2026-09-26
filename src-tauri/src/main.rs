// Release builds use the GUI subsystem, so Windows opens no console window beside the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    keytriage_lib::run();
}
