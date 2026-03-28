// builtin
use std::error::Error;

// external
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};

// internal

pub fn transcribe_audio(audio: Vec<f32>) -> Result<(), Box<dyn Error>> {
    let mut parakeet = ParakeetTDT::from_pretrained("./model/tdt", None)?;
    let result = parakeet.transcribe_samples(audio, 16000, 1, Some(TimestampMode::Sentences))?;
    println!("{}", result.text);

    for token in result.tokens {
        println!("[{:.3}s - {:.3}s] {}", token.start, token.end, token.text);
    }

    Ok(())
}
