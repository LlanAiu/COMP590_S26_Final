// builtin

// external

// internal
use crate::{ollama::send_message_ollama, transcribe::record_audio_to_pcm};

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

#[tauri::command(async)]
pub async fn send_message(message: String) -> String {
    println!("Sending message to Ollama...");

    let res = send_message_ollama(message).await;

    match res {
        Ok(response) => response,
        Err(err) => {
            println!("{}", err);
            "".into()
        }
    }
}
