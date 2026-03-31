// builtin

// external
use cpal::SupportedStreamConfig;

use crate::archives::transcription::constants::{SAMPLER_CHANNEL_SIZE, TRANSCRIBER_CHANNE_SIZE};
// internal
use crate::archives::transcription::{
    constants::TRANSCRIPTION_DESIRED_HZ, downsampler::Downsampler,
};
use crate::error::TranscriptionError;
use crate::{archives::transcription::AudioTranscriber, globals::Transcript};
use crate::{
    archives::{transcription::recorder::AudioRecorder, utils::chunk_channel::ChunkChannel},
    globals::Chunk,
};

pub struct ParakeetTranscriber {
    recorder: AudioRecorder,
    downsampler: Downsampler,

    sampler_channel: Option<ChunkChannel<Chunk>>,
    transcriber_channel: Option<ChunkChannel<Chunk>>,
}

impl ParakeetTranscriber {
    pub fn new() -> Result<ParakeetTranscriber, TranscriptionError> {
        let recorder: AudioRecorder = AudioRecorder::new()?;
        let downsampler: Downsampler = Downsampler::new(TRANSCRIPTION_DESIRED_HZ);

        Ok(ParakeetTranscriber {
            recorder,
            downsampler,
            sampler_channel: None,
            transcriber_channel: None,
        })
    }
}

impl AudioTranscriber for ParakeetTranscriber {
    fn start_record_audio(&mut self) -> Result<(), TranscriptionError> {
        let config: SupportedStreamConfig = self.recorder.start_recording()?;

        let sampler_channel: ChunkChannel<Chunk> = ChunkChannel::<Chunk>::new(SAMPLER_CHANNEL_SIZE);

        let transcriber_channel: ChunkChannel<Chunk> =
            ChunkChannel::<Chunk>::new(TRANSCRIBER_CHANNE_SIZE);

        self.recorder
            .setup_downstream(sampler_channel.get_sender())?;
        self.downsampler.setup_stream(
            config,
            sampler_channel.get_receiver(),
            transcriber_channel.get_sender(),
        );

        todo!()
    }

    fn stop_record_audio(&mut self) -> Result<(), TranscriptionError> {
        todo!()
    }

    fn get_transcript(&self) -> Result<Transcript, TranscriptionError> {
        todo!()
    }
}
