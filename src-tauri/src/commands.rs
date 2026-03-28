// builtin

// external

// internal
use crate::transcribe::record_audio_to_pcm;

#[tauri::command]
pub fn start_audio_recording() {
    println!("Starting audio recording...");
    let res = record_audio_to_pcm();

    match res {
        Ok(_) => {}
        Err(err) => {
            println!("{}", err)
        }
    };
}
