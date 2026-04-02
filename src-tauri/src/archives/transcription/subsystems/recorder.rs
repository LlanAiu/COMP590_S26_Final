use std::mem::take;
// builtin
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, sleep, JoinHandle};
use std::time::Duration;

// external
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, FromSample, Host, Sample, SampleFormat, Stream, SupportedInputConfigs,
    SupportedStreamConfig,
};
use crossbeam_channel::{Sender, TrySendError};
use ringbuf::storage::Heap;
use ringbuf::{traits::*, CachingCons, CachingProd, HeapRb, SharedRb};

// internal
use crate::archives::transcription::constants::{
    TRANSCRIPTION_DESIRED_HZ, WAV_BUFFER_SIZE, WAV_CHUNK_DURATION,
};
use crate::error::TranscriptionError;
use crate::globals::Chunk;

type RingBufferProducer = CachingProd<Arc<SharedRb<Heap<f32>>>>;
type RingBufferConsumer = CachingCons<Arc<SharedRb<Heap<f32>>>>;

pub struct AudioRecorder {
    device: Device,
    chunk_size: usize,
    consumer: Option<RingBufferConsumer>,
    stream: Option<Stream>,
    worker: Option<JoinHandle<()>>,
    shutdown: Option<Arc<AtomicBool>>,
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
            chunk_size: WAV_BUFFER_SIZE,
            consumer: None,
            stream: None,
            worker: None,
            shutdown: None,
        })
    }

    pub fn setup_downstream(&mut self, sender: Sender<Vec<f32>>) -> Result<(), TranscriptionError> {
        let consumer: Option<RingBufferConsumer> = take(&mut self.consumer);
        if let Some(cons) = consumer {
            let shutdown: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
            let shutdown_thread: Arc<AtomicBool> = Arc::clone(&shutdown);

            let handle: JoinHandle<()> =
                spawn_downstream_thread(cons, sender, self.chunk_size, shutdown_thread);
            self.worker = Some(handle);
            self.shutdown = Some(shutdown);

            return Ok(());
        }

        Err(TranscriptionError::InternalError(
            "No consumer found for audio recorder!".into(),
        ))
    }

    pub fn start_recording(&mut self) -> Result<SupportedStreamConfig, TranscriptionError> {
        let config: SupportedStreamConfig = self.get_input_stream_config()?;

        let chunk_size = config.sample_rate() * WAV_CHUNK_DURATION;
        let ring_buffer_size = usize::pow(2, u32::ilog2(chunk_size) + 1u32);

        self.chunk_size = chunk_size as usize;

        let ring_buffer = HeapRb::<f32>::new(ring_buffer_size);

        let (prod, cons): (RingBufferProducer, RingBufferConsumer) = ring_buffer.split();
        self.consumer = Some(cons);

        let stream: Stream = self.build_input_stream(config.clone(), prod)?;

        stream
            .play()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        self.stream = Some(stream);

        Ok(config)
    }

    pub fn stop_recording(&mut self) -> Result<(), TranscriptionError> {
        if let Some(stream) = self.stream.take() {
            stream
                .pause()
                .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;
            drop(stream);
        }

        sleep(Duration::from_millis(20));

        if let Some(shutdown) = self.shutdown.take() {
            shutdown.store(true, Ordering::SeqCst);
        }

        if let Some(handle) = self.worker.take() {
            if let Err(_) = handle.join() {
                return Err(TranscriptionError::ShutdownError(
                    "[RECORDER] Failed to close audio buffer thread".into(),
                ));
            }
        }

        Ok(())
    }

    fn get_input_stream_config(&self) -> Result<SupportedStreamConfig, TranscriptionError> {
        let ranges: SupportedInputConfigs = self
            .device
            .supported_input_configs()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        choose_input_config(ranges, TRANSCRIPTION_DESIRED_HZ)
    }

    fn build_input_stream(
        &mut self,
        config: SupportedStreamConfig,
        producer: RingBufferProducer,
    ) -> Result<Stream, TranscriptionError> {
        let channels: usize = config.config().channels.into();
        let sample_format = config.sample_format();

        build_input_for_sample_format(&self.device, sample_format, &config, channels, producer)
    }
}

fn spawn_downstream_thread(
    mut consumer: RingBufferConsumer,
    sender: Sender<Chunk>,
    chunk_size: usize,
    shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut tmp: Vec<f32> = Vec::with_capacity(chunk_size);
        loop {
            if shutdown.load(Ordering::Relaxed) {
                while let Some(s) = consumer.try_pop() {
                    tmp.push(s);
                    if tmp.len() >= chunk_size {
                        let chunk: Vec<f32> = take(&mut tmp);
                        match sender.try_send(chunk) {
                            Ok(()) => {}
                            Err(err) => match err {
                                TrySendError::Full(_) => {
                                    eprintln!("chunk channel full while draining, dropping chunk");
                                }
                                TrySendError::Disconnected(_) => {
                                    return;
                                }
                            },
                        }
                        tmp.reserve(chunk_size);
                    }
                }

                if !tmp.is_empty() {
                    let chunk: Vec<f32> = take(&mut tmp);
                    match sender.try_send(chunk) {
                        Ok(()) => {}
                        Err(err) => match err {
                            TrySendError::Full(_) => {
                                eprintln!("chunk channel full while draining, dropping chunk");
                            }
                            TrySendError::Disconnected(_) => {
                                return;
                            }
                        },
                    }
                }
                return;
            }

            while tmp.len() < chunk_size {
                match consumer.try_pop() {
                    Some(s) => tmp.push(s),
                    None => {
                        if shutdown.load(Ordering::Relaxed) {
                            break;
                        }
                        sleep(Duration::from_millis(2));
                    }
                }
            }

            if tmp.is_empty() {
                continue;
            }

            let chunk: Vec<f32> = take(&mut tmp);

            match sender.try_send(chunk) {
                Ok(()) => {}
                Err(err) => match err {
                    TrySendError::Full(_) => {
                        eprintln!("chunk channel full, dropping chunk");
                    }
                    TrySendError::Disconnected(_) => {
                        break;
                    }
                },
            }
            tmp.reserve(chunk_size);
        }
    })
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
    let stream = if sample_format == SampleFormat::F32 {
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
    .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

    return Ok(stream);
}

fn process_and_append<T: Sample>(data: &[T], channels: usize, buffer: &mut RingBufferProducer)
where
    f32: FromSample<T>,
{
    if channels == 1 {
        if !data.is_empty() {
            for &s in data.iter() {
                if buffer.try_push(f32::from_sample(s)).is_err() {
                    eprintln!("Failed to push sample to buffer");
                }
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
            if buffer.try_push(sum / (channels as f32)).is_err() {
                eprintln!("Failed to push sample to buffer");
            }
        }
    }
}
