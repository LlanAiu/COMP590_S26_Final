// builtin
use std::sync::{Arc, Mutex};
use std::thread;

// external
use tauri::{AppHandle, Manager};

// internal
use crate::archives::Archives;

type ArchiveRef = Arc<Mutex<Archives>>;

#[tauri::command]
pub fn start_audio_recording(app: AppHandle) {
    println!("Starting audio recording...");
    let state = app.state::<ArchiveRef>().clone();
    let state_ref = Arc::clone(&state);

    thread::spawn(move || {
        let mut guard = state_ref.lock().unwrap();

        if let Err(err) = guard.start_audio_recording() {
            eprintln!("{}", err);
        }
    });
}

#[tauri::command]
pub fn stop_audio_recording(app: AppHandle) {
    println!("Stopping audio recording...");
    let state = app.state::<ArchiveRef>().clone();
    let state_ref = Arc::clone(&state);

    thread::spawn(move || {
        let mut guard = state_ref.lock().unwrap();

        if let Err(err) = guard.stop_audio_recording() {
            eprintln!("{}", err);
        }
    });
}
