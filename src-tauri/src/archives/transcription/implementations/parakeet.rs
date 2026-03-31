// builtin

// external
use cpal::SupportedStreamConfig;
use crossbeam_channel::bounded;

// internal
use crate::archives::transcription::constants::{
    AUDIO_CHANNEL_SIZE, SAMPLED_CHANNEL_SIZE, TRANSCRIPTION_DESIRED_HZ,
};
use crate::archives::transcription::subsystems::{
    downsampler::Downsampler, parakeet_module::ParakeetModule, recorder::AudioRecorder,
};
use crate::archives::transcription::AudioTranscriber;
use crate::error::TranscriptionError;
use crate::globals::{Chunk, Transcript};

pub struct ParakeetTranscriber {
    recorder: AudioRecorder,
    downsampler: Downsampler,
    parakeet: ParakeetModule,
}

impl ParakeetTranscriber {
    pub fn new() -> Result<ParakeetTranscriber, TranscriptionError> {
        let recorder: AudioRecorder = AudioRecorder::new()?;
        let downsampler: Downsampler = Downsampler::new(TRANSCRIPTION_DESIRED_HZ);
        let parakeet: ParakeetModule = ParakeetModule::new()?;

        Ok(ParakeetTranscriber {
            recorder,
            downsampler,
            parakeet,
        })
    }
}

impl AudioTranscriber for ParakeetTranscriber {
    fn start_record_audio(&mut self) -> Result<(), TranscriptionError> {
        let config: SupportedStreamConfig = self.recorder.start_recording()?;

        let (audio_tx, audio_rx) = bounded::<Chunk>(AUDIO_CHANNEL_SIZE);

        let (sampled_tx, sampled_rx) = bounded::<Chunk>(SAMPLED_CHANNEL_SIZE);

        self.recorder.setup_downstream(audio_tx)?;

        self.downsampler.setup_stream(config, audio_rx, sampled_tx);

        Ok(())
    }

    fn stop_record_audio(&mut self) -> Result<(), TranscriptionError> {
        todo!()
    }

    fn get_transcript(&self) -> Result<Transcript, TranscriptionError> {
        todo!()
    }
}
