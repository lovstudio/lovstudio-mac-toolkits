// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--helper") {
        if let Err(error) = lovstudio_mac_toolkits_lib::run_helper() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    lovstudio_mac_toolkits_lib::run();
}
