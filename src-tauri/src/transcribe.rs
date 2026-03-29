// builtin
use std::mem::take;
use std::sync::{Arc, Mutex, MutexGuard};

// external
use audioadapter_buffers::owned::InterleavedOwned;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, SupportedInputConfigs, SupportedStreamConfig};
use cpal::{FromSample, Sample};
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
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

    let supported_config = choose_input_config(ranges, 22_500, Some(SampleFormat::I16))?;

    let audio_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let callback_buffer_ref: Arc<Mutex<Vec<f32>>> = audio_buffer.clone();

    let channels = supported_config.config().channels as usize;

    let sample_format = supported_config.sample_format();

    let cb_ref = callback_buffer_ref.clone();

    fn process_and_append<T: Sample>(data: &[T], channels: usize, cb: &Arc<Mutex<Vec<f32>>>)
    where
        f32: FromSample<T>,
    {
        if channels == 1 {
            if !data.is_empty() {
                let mut tmp: Vec<f32> = Vec::with_capacity(data.len());
                for &s in data.iter() {
                    tmp.push(f32::from_sample(s));
                }
                if let Ok(mut buffer) = cb.lock() {
                    buffer.extend_from_slice(&tmp);
                }
            }
        } else if channels > 1 {
            let frames = data.len() / channels;
            if frames == 0 {
                return;
            }
            let mut mono: Vec<f32> = Vec::with_capacity(frames);
            for frame_idx in 0..frames {
                let base = frame_idx * channels;
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    sum += f32::from_sample(data[base + ch]);
                }
                mono.push(sum / (channels as f32));
            }
            if let Ok(mut buffer) = cb.lock() {
                buffer.extend_from_slice(&mono);
            }
        }
    }

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &supported_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &cb_ref)
            },
            move |err| println!("{:?}", err),
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &supported_config.config(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &cb_ref)
            },
            move |err| println!("{:?}", err),
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &supported_config.config(),
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &cb_ref)
            },
            move |err| println!("{:?}", err),
            None,
        ),
        _ => {
            return Err(TranscriptionError::InternalError(
                "Unsupported sample format".to_string(),
            ))
        }
    }
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

    let sample_rate = supported_config.config().sample_rate;
    transcribe_audio(audio, sample_rate)?;

    Ok(())
}

pub fn transcribe_audio(audio: Vec<f32>, sample_rate: u32) -> Result<(), TranscriptionError> {
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

    let processed_audio = if sample_rate == 16_000 {
        audio
    } else if sample_rate < 16_000 {
        return Err(TranscriptionError::InternalError(
            "[RECORDER] Input sample rate is below required 16 kHz".to_string(),
        ));
    } else {
        resample_rubato(&audio, sample_rate as usize, 16_000usize)?
    };

    let result = parakeet
        .transcribe_samples(processed_audio, 16000, 1, Some(TimestampMode::Sentences))
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    println!("{}", result.text);

    for token in result.tokens {
        println!("[{:.3}s - {:.3}s] {}", token.start, token.end, token.text);
    }

    Ok(())
}

fn resample_rubato(
    input: &Vec<f32>,
    from_hz: usize,
    to_hz: usize,
) -> Result<Vec<f32>, TranscriptionError> {
    if from_hz == to_hz {
        return Ok(input.clone());
    }

    let ratio = to_hz as f64 / from_hz as f64;

    let frames = input.len();

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        oversampling_factor: 128,
        interpolation: SincInterpolationType::Cubic,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler =
        Async::<f32>::new_sinc(ratio, 1.1, &params, frames.max(1), 1, FixedAsync::Input).map_err(
            |e| TranscriptionError::InternalError(format!("[RESAMPLE] rubato init error: {:?}", e)),
        )?;

    let in_buf = InterleavedOwned::new_from(input.clone(), 1, frames).map_err(|e| {
        TranscriptionError::InternalError(format!("[RESAMPLE] input buffer error: {:?}", e))
    })?;

    let out = resampler.process(&in_buf, 0, None).map_err(|e| {
        TranscriptionError::InternalError(format!("[RESAMPLE] rubato process error: {:?}", e))
    })?;

    Ok(out.take_data())
}

fn choose_input_config(
    ranges: SupportedInputConfigs,
    target_hz: u32,
    preferred_format: Option<SampleFormat>,
) -> Result<SupportedStreamConfig, TranscriptionError> {
    let mut best_f32: Option<(SupportedStreamConfig, u32)> = None;
    let mut best_any: Option<(SupportedStreamConfig, u32)> = None;
    let mut best_pref: Option<(SupportedStreamConfig, u32)> = None;

    for range in ranges {
        let min = range.min_sample_rate();
        let max = range.max_sample_rate();

        if max < target_hz {
            continue;
        }

        let chosen = if min >= target_hz { min } else { target_hz };

        let cfg = range.with_sample_rate(chosen);
        let diff = if chosen >= target_hz {
            chosen - target_hz
        } else {
            0
        };

        if let Some(pref) = preferred_format {
            if cfg.sample_format() == pref {
                match &best_pref {
                    Some((_, best_diff)) if *best_diff <= diff => {}
                    _ => best_pref = Some((cfg.clone(), diff)),
                }
            }
        }

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

    if let Some((cfg, _)) = best_pref {
        return Ok(cfg);
    }

    if let Some((cfg, _)) = best_f32 {
        return Ok(cfg);
    }
    if let Some((cfg, _)) = best_any {
        return Ok(cfg);
    }

    Err(TranscriptionError::InternalError(
        "[RECORDER] No input sample range supports >= target sample rate".to_string(),
    ))
}
