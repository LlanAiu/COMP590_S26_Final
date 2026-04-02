// builtin
use std::sync::{Arc, Mutex};

// external
use tauri::{AppHandle, Manager};

// internal
use crate::archives::Archives;

type ArchiveRef = Arc<Mutex<Archives>>;

#[tauri::command]
pub fn start_audio_recording(app: AppHandle) {
    println!("Starting audio recording...");
    let state = app.state::<ArchiveRef>();
    let mut guard = state.lock().unwrap();

    match guard.start_audio_recording() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    };
}

#[tauri::command]
pub fn stop_audio_recording(app: AppHandle) {
    println!("Stopping audio recording...");
    let state = app.state::<ArchiveRef>();
    let mut guard = state.lock().unwrap();

    match guard.stop_audio_recording() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    };
}
