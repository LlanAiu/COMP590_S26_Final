// builtin

// external

// internal

// modules
pub mod parakeet_impl;
pub mod recorder;
pub mod test_impl;

pub trait AudioTranscriber {
    fn start_record_audio(&mut self);

    fn stop_record_audio(&mut self);

    fn get_transcript(&self) -> Vec<String>;
}
