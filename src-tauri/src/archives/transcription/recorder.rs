use std::mem::take;
// builtin
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;

// external
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, FromSample, Host, Sample, SampleFormat, Stream, SupportedInputConfigs,
    SupportedStreamConfig,
};
use crossbeam_channel::Sender;
use ringbuf::storage::Heap;
use ringbuf::{traits::*, CachingCons, CachingProd, HeapRb, SharedRb};

// internal
use crate::archives::transcription::constants::{TRANSCRIPTION_DESIRED_HZ, WAV_BUFFER_SIZE};
use crate::error::TranscriptionError;

type RingBufferProducer = CachingProd<Arc<SharedRb<Heap<f32>>>>;
type RingBufferConsumer = CachingCons<Arc<SharedRb<Heap<f32>>>>;

pub struct AudioRecorder {
    device: Device,
    consumer: Option<RingBufferConsumer>,
    stream: Option<Stream>,
}

impl AudioRecorder {
    pub fn new() -> Result<AudioRecorder, TranscriptionError> {
        let host: Host = cpal::default_host();

        let device: Device = match host.default_input_device() {
            Some(device) => device,
            None => return Err(TranscriptionError::NoDevicesFound),
        };

        Ok(AudioRecorder {
            device,
            consumer: None,
            stream: None,
        })
    }

    pub fn setup_downstream(&mut self, sender: Sender<Vec<f32>>) -> Result<(), TranscriptionError> {
        let consumer = take(&mut self.consumer);
        if let Some(mut cons) = consumer {
            thread::spawn(move || {
                let mut tmp: Vec<f32> = Vec::with_capacity(WAV_BUFFER_SIZE);
                loop {
                    while tmp.len() < WAV_BUFFER_SIZE {
                        match cons.try_pop() {
                            Some(s) => tmp.push(s),
                            None => {
                                sleep(Duration::from_millis(2));
                            }
                        }
                    }

                    let chunk: Vec<f32> = take(&mut tmp);

                    match sender.try_send(chunk) {
                        Ok(()) => {}
                        Err(_) => {
                            eprintln!("chunk channel full, dropping chunk");
                        }
                    }
                    tmp.reserve(WAV_BUFFER_SIZE);
                }
            });

            return Ok(());
        }

        Err(TranscriptionError::InternalError(
            "No consumer found for audio recorder!".into(),
        ))
    }

    pub fn start_recording(&mut self) -> Result<(), TranscriptionError> {
        let ring_buffer = HeapRb::<f32>::new(WAV_BUFFER_SIZE);

        let (prod, cons) = ring_buffer.split();
        self.consumer = Some(cons);

        let stream: Stream = self.build_input_stream(prod)?;

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

    fn build_input_stream(
        &mut self,
        producer: RingBufferProducer,
    ) -> Result<Stream, TranscriptionError> {
        let ranges: SupportedInputConfigs = self
            .device
            .supported_input_configs()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        let supported_config = choose_input_config(ranges, TRANSCRIPTION_DESIRED_HZ)?;

        let channels: usize = supported_config.config().channels.into();
        let sample_format = supported_config.sample_format();

        build_input_for_sample_format(
            &self.device,
            sample_format,
            &supported_config,
            channels,
            producer,
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
    buffer: RingBufferProducer,
) -> Result<Stream, TranscriptionError> {
    if sample_format == SampleFormat::F32 {
        let mut buffer = buffer;
        device.build_input_stream(
            &supported_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &mut buffer)
            },
            move |err| println!("{:?}", err),
            None,
        )
    } else if sample_format == SampleFormat::I16 {
        let mut buffer = buffer;
        device.build_input_stream(
            &supported_config.config(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &mut buffer)
            },
            move |err| println!("{:?}", err),
            None,
        )
    } else if sample_format == SampleFormat::U16 {
        let mut buffer = buffer;
        device.build_input_stream(
            &supported_config.config(),
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                process_and_append(data, channels, &mut buffer)
            },
            move |err| println!("{:?}", err),
            None,
        )
    } else {
        return Err(TranscriptionError::InternalError(
            "Unsupported sample format".to_string(),
        ));
    }
    .map_err(|err| TranscriptionError::InternalError(err.to_string()))
}

fn process_and_append<T: Sample>(data: &[T], channels: usize, buffer: &mut RingBufferProducer)
where
    f32: FromSample<T>,
{
    if channels == 1 {
        if !data.is_empty() {
            for &s in data.iter() {
                buffer.try_push(f32::from_sample(s));
            }
        }
    } else if channels > 1 {
        let frames = data.len() / channels;
        if frames == 0 {
            return;
        }
        for frame_idx in 0..frames {
            let base = frame_idx * channels;
            let mut sum = 0.0f32;
            for ch in 0..channels {
                sum += f32::from_sample(data[base + ch]);
            }
            buffer.try_push(sum / (channels as f32));
        }
    }
}
