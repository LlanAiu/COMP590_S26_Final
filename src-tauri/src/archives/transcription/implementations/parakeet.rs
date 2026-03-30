// builtin
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// external
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};

// internal
use crate::archives::transcription::downsampler::Downsampler;
use crate::archives::transcription::recorder::AudioRecorder;
use crate::archives::transcription::AudioTranscriber;
use crate::archives::utils::chunk_queue::ChunkQueue;
use crate::error::TranscriptionError;

pub struct ParakeetTranscriber {
    // Subsystems
    recorder: Option<AudioRecorder>,
    downsampler: Option<Downsampler>,

    // Queues
    raw_queue: Arc<Mutex<ChunkQueue<f32>>>,
    downsampled_queue: Arc<Mutex<ChunkQueue<f32>>>,
    out_queue: Arc<Mutex<ChunkQueue<String>>>,

    // transcripts storage and control
    transcripts: Arc<Mutex<Vec<String>>>,
    running: Arc<AtomicBool>,
    downsampler_handle: Option<JoinHandle<()>>,
    parakeet_handle: Option<JoinHandle<()>>,
}

impl ParakeetTranscriber {
    pub fn new() -> ParakeetTranscriber {
        ParakeetTranscriber {
            recorder: None,
            downsampler: None,
            raw_queue: Arc::new(Mutex::new(ChunkQueue::new())),
            downsampled_queue: Arc::new(Mutex::new(ChunkQueue::new())),
            out_queue: Arc::new(Mutex::new(ChunkQueue::new())),
            transcripts: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            downsampler_handle: None,
            parakeet_handle: None,
        }
    }

    pub fn set_output_queue(
        &mut self,
        queue_ref: &Arc<Mutex<ChunkQueue<String>>>,
    ) -> Result<(), TranscriptionError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(TranscriptionError::InternalError(
                "Cannot set output queue while running".into(),
            ));
        }
        self.out_queue = Arc::clone(queue_ref);
        Ok(())
    }
}

impl AudioTranscriber for ParakeetTranscriber {
    fn start_record_audio(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        let running = Arc::clone(&self.running);
        running.store(true, Ordering::SeqCst);
        if self.recorder.is_none() {
            match AudioRecorder::new() {
                Ok(r) => {
                    let _ = r.set_output_queue(&self.raw_queue);
                    self.recorder = Some(r);
                }
                Err(err) => {
                    eprintln!("[PARAKEET] failed to create recorder: {:?}", err);
                    self.running.store(false, Ordering::SeqCst);
                    return;
                }
            }
        }

        if self.downsampler.is_none() {
            let mut ds = Downsampler::new(16_000);
            ds.set_input_queue(&self.raw_queue);
            if let Err(err) = ds.set_output_queue(&self.downsampled_queue) {
                eprintln!(
                    "[PARAKEET] failed to set downsampler output queue: {:?}",
                    err
                );
                self.running.store(false, Ordering::SeqCst);
                return;
            }
            self.downsampler = Some(ds);
        }

        if let Some(rec) = &mut self.recorder {
            if let Err(err) = rec.start_recording() {
                eprintln!("[PARAKEET] failed to start recorder: {:?}", err);
                self.running.store(false, Ordering::SeqCst);
                return;
            }
        }

        if let Some(ds) = self.downsampler.take() {
            let running_ds = Arc::clone(&self.running);
            let down_handle = thread::spawn(move || {
                let ds = ds;
                while running_ds.load(Ordering::SeqCst) {
                    if let Err(err) = ds.process_next() {
                        // Non-fatal: log and continue
                        eprintln!("[DOWNSAMPLER] process_next error: {:?}", err);
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            });
            self.downsampler_handle = Some(down_handle);
        }

        let down_q = Arc::clone(&self.downsampled_queue);
        let out_q = Arc::clone(&self.out_queue);
        let transcripts = Arc::clone(&self.transcripts);
        let running_p = Arc::clone(&self.running);

        let par_handle = thread::spawn(move || {
            let model_dir = match std::env::var("TAURI_MODEL_DIR") {
                Ok(dir) => dir,
                Err(_) => {
                    eprintln!("[PARAKEET] TAURI_MODEL_DIR not set");
                    running_p.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut parakeet = match ParakeetTDT::from_pretrained(&model_dir, None) {
                Ok(m) => m,
                Err(err) => {
                    eprintln!("[PARAKEET] failed to load model: {:?}", err);
                    running_p.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut acc: Vec<f32> = Vec::new();
            let target_hz = 16_000usize;

            while running_p.load(Ordering::SeqCst) {
                let mut took_chunk = false;
                if let Ok(mut guard) = down_q.lock() {
                    if !guard.is_empty() {
                        if let Some(chunk) = guard.next_chunk() {
                            acc.extend(chunk);
                            took_chunk = true;
                        }
                    }
                }

                if !took_chunk {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                if acc.len() >= target_hz / 2 {
                    let mut to_transcribe = Vec::new();
                    std::mem::swap(&mut to_transcribe, &mut acc);

                    match parakeet.transcribe_samples(
                        to_transcribe.clone(),
                        target_hz as u32,
                        1,
                        Some(TimestampMode::Sentences),
                    ) {
                        Ok(result) => {
                            let text = result.text;
                            if !text.is_empty() {
                                if let Ok(mut tguard) = transcripts.lock() {
                                    tguard.push(text.clone());
                                }
                                if let Ok(mut og) = out_q.lock() {
                                    og.add_chunk(vec![text]);
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("[PARAKEET] transcribe error: {:?}", err);
                        }
                    }
                }
            }
        });

        self.parakeet_handle = Some(par_handle);
    }

    fn stop_record_audio(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.parakeet_handle.take() {
            let _ = h.join();
        }
        if let Some(h) = self.downsampler_handle.take() {
            let _ = h.join();
        }

        if let Some(rec) = &mut self.recorder {
            let _ = rec.stop_recording();
        }
    }

    fn get_transcript(&self) -> Vec<String> {
        if let Ok(guard) = self.transcripts.lock() {
            guard.clone()
        } else {
            Vec::new()
        }
    }
}
