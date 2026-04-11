// builtin
use std::sync::{Arc, Mutex};

// external

// internal
use crate::archives::Archives;

// modules
pub mod archives;
pub mod commands;
pub mod error;
pub mod globals;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let archives: Archives = match Archives::new() {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Failed to boot application with error: {:?}", err);
            return;
        }
    };

    let archive_ref: Arc<Mutex<Archives>> = Arc::new(Mutex::new(archives));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(archive_ref)
        .invoke_handler(tauri::generate_handler![
            commands::start_audio_recording,
            commands::stop_audio_recording,
            commands::create_volume,
            commands::read_volume,
            commands::edit_volume,
            commands::delete_volume,
            commands::list_volumes,
            commands::nest_volume,
            commands::flatten_volume,
            commands::merge_volumes,
            commands::split_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
