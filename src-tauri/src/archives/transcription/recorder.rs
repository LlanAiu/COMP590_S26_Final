// builtin
use std::sync::{Arc, Mutex};

// external
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, FromSample, Host, Sample, SampleFormat, Stream, SupportedInputConfigs,
    SupportedStreamConfig,
};

// internal
use crate::archives::transcription::constants::{TRANSCRIPTION_DESIRED_HZ, WAV_BUFFER_SIZE};
use crate::archives::utils::chunk_buffer::ChunkBuffer;
use crate::archives::utils::chunk_queue::ChunkQueue;
use crate::error::TranscriptionError;

pub struct AudioRecorder {
    device: Device,
    buffer: Arc<Mutex<ChunkBuffer<f32>>>,
    stream: Option<Stream>,
}

impl AudioRecorder {
    pub fn new() -> Result<AudioRecorder, TranscriptionError> {
        let host: Host = cpal::default_host();

        let device: Device = match host.default_input_device() {
            Some(device) => device,
            None => return Err(TranscriptionError::NoDevicesFound),
        };

        let buffer: ChunkBuffer<f32> = ChunkBuffer::new(WAV_BUFFER_SIZE);

        Ok(AudioRecorder {
            device,
            buffer: Arc::new(Mutex::new(buffer)),
            stream: None,
        })
    }

    pub fn set_output_queue(
        &self,
        queue_ref: &Arc<Mutex<ChunkQueue<f32>>>,
    ) -> Result<(), TranscriptionError> {
        let mut guard = self
            .buffer
            .lock()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        guard.set_out_queue(queue_ref);

        Ok(())
    }

    pub fn start_recording(&mut self) -> Result<(), TranscriptionError> {
        let stream: Stream = self.build_input_stream()?;

        stream
            .play()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        self.stream = Some(stream);

        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<(), TranscriptionError> {
        if let Some(stream) = self.stream.take() {
            stream
                .pause()
                .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;
            drop(stream);
        }
        Ok(())
    }

    fn build_input_stream(&self) -> Result<Stream, TranscriptionError> {
        let ranges: SupportedInputConfigs = self
            .device
            .supported_input_configs()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        let supported_config = choose_input_config(ranges, TRANSCRIPTION_DESIRED_HZ)?;

        let callback_buffer_ref: Arc<Mutex<ChunkBuffer<f32>>> = Arc::clone(&self.buffer);

        let channels: usize = supported_config.config().channels.into();
        let sample_format = supported_config.sample_format();

        build_input_for_sample_format(
            &self.device,
            sample_format,
            &supported_config,
            channels,
            callback_buffer_ref,
        )
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

fn build_input_for_sample_format(
    device: &Device,
    sample_format: SampleFormat,
    supported_config: &SupportedStreamConfig,
    channels: usize,
    buffer: Arc<Mutex<ChunkBuffer<f32>>>,
) -> Result<Stream, TranscriptionError> {
    match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &supported_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &buffer)
            },
            move |err| println!("{:?}", err),
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &supported_config.config(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &buffer)
            },
            move |err| println!("{:?}", err),
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &supported_config.config(),
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &buffer)
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

fn process_and_append<T: Sample>(data: &[T], channels: usize, buffer: &Arc<Mutex<ChunkBuffer<f32>>>)
where
    f32: FromSample<T>,
{
    if channels == 1 {
        if !data.is_empty() {
            let mut tmp: Vec<f32> = Vec::with_capacity(data.len());
            for &s in data.iter() {
                tmp.push(f32::from_sample(s));
            }
            if let Ok(mut buffer) = buffer.lock() {
                let _ = buffer.add_slice(&tmp);
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
        if let Ok(mut buffer) = buffer.lock() {
            let _ = buffer.add_slice(&mono);
        }
    }
}
