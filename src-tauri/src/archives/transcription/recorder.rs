// builtin
use std::sync::{Arc, Mutex};

// external
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{
    Device, FromSample, Host, Sample, SampleFormat, Stream, SupportedInputConfigs,
    SupportedStreamConfig,
};

// internal
use crate::archives::transcription::constants::TRANSCRIPTION_DESIRED_HZ;
use crate::error::TranscriptionError;

pub struct AudioRecorder {
    device: Device,
}

impl AudioRecorder {
    pub fn new() -> Result<AudioRecorder, TranscriptionError> {
        let host: Host = cpal::default_host();

        let device: Device = match host.default_input_device() {
            Some(device) => device,
            None => return Err(TranscriptionError::NoDevicesFound),
        };

        Ok(AudioRecorder { device })
    }

    pub fn build_input_stream(
        &self,
        audio_buffer: &Arc<Mutex<Vec<f32>>>,
    ) -> Result<Stream, TranscriptionError> {
        let ranges: SupportedInputConfigs = self
            .device
            .supported_input_configs()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        let supported_config = choose_input_config(ranges, TRANSCRIPTION_DESIRED_HZ)?;

        let callback_buffer_ref: Arc<Mutex<Vec<f32>>> = Arc::clone(audio_buffer);

        let channels: usize = supported_config.config().channels.into();
        let sample_format = supported_config.sample_format();

        match sample_format {
            SampleFormat::F32 => self.device.build_input_stream(
                &supported_config.config(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    process_and_append(data, channels, &callback_buffer_ref)
                },
                move |err| println!("{:?}", err),
                None,
            ),
            SampleFormat::I16 => self.device.build_input_stream(
                &supported_config.config(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    process_and_append(data, channels, &callback_buffer_ref)
                },
                move |err| println!("{:?}", err),
                None,
            ),
            SampleFormat::U16 => self.device.build_input_stream(
                &supported_config.config(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    process_and_append(data, channels, &callback_buffer_ref)
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
        .map_err(|err| TranscriptionError::InternalError(err.to_string()))
    }
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

    Err(TranscriptionError::UnsupportedSampleRange(target_hz))
}

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
