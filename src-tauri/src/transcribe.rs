// builtin
use std::mem::take;
use std::sync::{Arc, Mutex, MutexGuard};

// external
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, SupportedInputConfigs, SupportedStreamConfig};
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};
use std::path::PathBuf;

// internal
use crate::error::TranscriptionError;

pub fn record_audio_to_pcm() -> Result<(), TranscriptionError> {
    let host: Host = cpal::default_host();

    let device: Device = match host.default_input_device() {
        Some(device) => device,
        None => {
            return Err(TranscriptionError::InternalError(
                "[RECORDER] No input device found on system".to_string(),
            ))
        }
    };

    let ranges: SupportedInputConfigs = device
        .supported_input_configs()
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    let supported_config = choose_input_config(ranges, 16_000)?;

    let audio_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let callback_buffer_ref: Arc<Mutex<Vec<f32>>> = audio_buffer.clone();

    let channels = supported_config.config().channels as usize;

    let stream = device
        .build_input_stream(
            &supported_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if channels == 1 {
                    if !data.is_empty() {
                        let mut local = Vec::with_capacity(data.len());
                        local.extend_from_slice(data);
                        if let Ok(mut buffer) = callback_buffer_ref.lock() {
                            buffer.extend_from_slice(&local);
                        }
                    }
                } else if channels > 1 {
                    let frames = data.len() / channels;
                    if frames == 0 {
                        return;
                    }
                    let mut mono: Vec<f32> = Vec::with_capacity(frames);
                    for frame_idx in 0..frames {
                        let mut sum = 0.0f32;
                        let base = frame_idx * channels;
                        for channel in 0..channels {
                            sum += data[base + channel];
                        }
                        mono.push(sum / (channels as f32));
                    }
                    if let Ok(mut buffer) = callback_buffer_ref.lock() {
                        buffer.extend_from_slice(&mono);
                    }
                }
            },
            move |err| println!("{:?}", err),
            None,
        )
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    stream
        .play()
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    std::thread::sleep(std::time::Duration::from_secs(3));

    drop(stream);

    println!("AUDIO CONFIG: {:?}", supported_config);

    let audio: Vec<f32> = {
        let mut guard: MutexGuard<Vec<f32>> = audio_buffer
            .lock()
            .map_err(|_| TranscriptionError::InternalError("Failed to lock buffer".into()))?;
        take(&mut *guard)
    };

    transcribe_audio(audio)?;

    Ok(())
}

pub fn transcribe_audio(audio: Vec<f32>) -> Result<(), TranscriptionError> {
    let model_path: PathBuf = match std::env::var("TAURI_MODEL_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            return Err(TranscriptionError::InternalError(
                "TDT model path not found in environment!".to_string(),
            ))
        }
    };

    let model_path_str = model_path.to_string_lossy().to_string();

    let mut parakeet = ParakeetTDT::from_pretrained(&model_path_str, None)
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;
    let result = parakeet
        .transcribe_samples(audio, 16000, 1, Some(TimestampMode::Sentences))
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    println!("{}", result.text);

    for token in result.tokens {
        println!("[{:.3}s - {:.3}s] {}", token.start, token.end, token.text);
    }

    Ok(())
}

fn choose_input_config(
    ranges: SupportedInputConfigs,
    target_hz: u32,
) -> Result<SupportedStreamConfig, TranscriptionError> {
    let mut best_f32: Option<(SupportedStreamConfig, u32)> = None;
    let mut best_any: Option<(SupportedStreamConfig, u32)> = None;

    for range in ranges {
        let min = range.min_sample_rate();
        let max = range.max_sample_rate();

        let chosen = if target_hz < min {
            min
        } else if target_hz > max {
            max
        } else {
            target_hz
        };
        let cfg = range.with_sample_rate(chosen);
        let diff = if chosen > target_hz {
            chosen - target_hz
        } else {
            target_hz - chosen
        };

        if cfg.sample_format() == SampleFormat::F32 {
            match &best_f32 {
                Some((_, best_diff)) if *best_diff <= diff => {}
                _ => best_f32 = Some((cfg.clone(), diff)),
            }
        }

        match &best_any {
            Some((_, best_diff)) if *best_diff <= diff => {}
            _ => best_any = Some((cfg.clone(), diff)),
        }
    }

    if let Some((cfg, _)) = best_f32 {
        return Ok(cfg);
    }
    if let Some((cfg, _)) = best_any {
        return Ok(cfg);
    }

    Err(TranscriptionError::InternalError(
        "[RECORDER] No input sample range configurations supported".to_string(),
    ))
}
