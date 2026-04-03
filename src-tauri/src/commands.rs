// builtin
use std::sync::{Arc, Mutex};
use std::thread;

// external
use tauri::{AppHandle, Manager};

// internal
use crate::archives::volumes::types::{
    CreateVolumeRequest, UpdateVolumeRequest, Volume, VolumeIndexEntry,
};
use crate::archives::volumes::VolumeDatabase;
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

#[tauri::command]
pub async fn create_volume(app: AppHandle, req: CreateVolumeRequest) -> Result<Volume, String> {
    let state = app.state::<ArchiveRef>().clone();
    let db = {
        let guard = state.lock().unwrap();
        guard.get_volume_database()
    };

    db.create_volume(req).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_volume(app: AppHandle, id: String) -> Result<Volume, String> {
    let state = app.state::<ArchiveRef>().clone();
    let db = {
        let guard = state.lock().unwrap();
        guard.get_volume_database()
    };

    db.read_volume(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn edit_volume(
    app: AppHandle,
    id: String,
    req: UpdateVolumeRequest,
) -> Result<Volume, String> {
    let state = app.state::<ArchiveRef>().clone();
    let db = {
        let guard = state.lock().unwrap();
        guard.get_volume_database()
    };

    db.edit_volume(&id, req).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_volume(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<ArchiveRef>().clone();
    let db = {
        let guard = state.lock().unwrap();
        guard.get_volume_database()
    };

    db.delete_volume(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_volumes(app: AppHandle) -> Result<Vec<VolumeIndexEntry>, String> {
    let state = app.state::<ArchiveRef>().clone();
    let db = {
        let guard = state.lock().unwrap();
        guard.get_volume_database()
    };

    db.list_index().await.map_err(|e| e.to_string())
}
