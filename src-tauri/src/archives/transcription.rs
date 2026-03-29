// builtin

// external

// internal

// modules
pub mod constants;
pub mod downsampler;
pub mod implementations;
pub mod recorder;

pub trait AudioTranscriber {
    fn start_record_audio(&mut self);

    fn stop_record_audio(&mut self);

    fn get_transcript(&self) -> Vec<String>;
}
