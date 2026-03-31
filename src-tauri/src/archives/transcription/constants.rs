// builtin

// external

// internal

pub const TRANSCRIPTION_DESIRED_HZ: u32 = 16_000;

pub const WAV_CHUNK_LENGTH: u32 = 8;
pub const WAV_BUFFER_SIZE: usize = (TRANSCRIPTION_DESIRED_HZ * WAV_CHUNK_LENGTH) as usize;

pub const RING_BUFFER_SIZE: usize = usize::pow(2, usize::ilog2(WAV_BUFFER_SIZE) + 1u32);
