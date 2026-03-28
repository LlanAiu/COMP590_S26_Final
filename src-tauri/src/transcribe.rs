// builtin

// external
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SupportedInputConfigs, SupportedStreamConfig};
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

    let mut supported_configs_range: SupportedInputConfigs = device
        .supported_input_configs()
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    let supported_config: SupportedStreamConfig = match supported_configs_range.next() {
        Some(config) => config.with_max_sample_rate(),
        None => {
            return Err(TranscriptionError::InternalError(
                "[RECORDER] No input sample range configurations supported".to_string(),
            ))
        }
    };

    let stream = device
        .build_input_stream(
            &supported_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
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
