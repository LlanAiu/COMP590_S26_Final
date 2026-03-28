// builtin

// external
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, SupportedInputConfigs, SupportedStreamConfig};
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};

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

    let stream = device
        .build_input_stream(
            &supported_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // TODO: Actually do something meaningful with the data
                println!("GOT AUDIO DATA: {:?}", data)
            },
            move |err| println!("{:?}", err),
            None,
        )
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    stream
        .play()
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("AUDIO CONFIG: {:?}", supported_config);

    Ok(())
}

pub fn transcribe_audio(audio: Vec<f32>) -> Result<(), TranscriptionError> {
    let mut parakeet = ParakeetTDT::from_pretrained("./model/tdt", None)
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
