use std::io::Cursor;
use hound::{WavReader, WavSpec, WavWriter, SampleFormat};
use tracing::info;

pub const DEFAULT_SAMPLE_RATE: u32 = 24000;
pub const DEFAULT_CHANNELS: u16 = 1;
pub const DEFAULT_BITS_PER_SAMPLE: u16 = 16;

pub fn get_standard_wav_spec() -> WavSpec {
    WavSpec {
        channels: DEFAULT_CHANNELS,
        sample_rate: DEFAULT_SAMPLE_RATE,
        bits_per_sample: DEFAULT_BITS_PER_SAMPLE,
        sample_format: SampleFormat::Int,
    }
}

/// Converts raw 16-bit PCM bytes (little-endian, 24kHz mono) into a valid RIFF WAV byte vector.
pub fn pcm_to_wav_bytes(pcm_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let spec = get_standard_wav_spec();
    let mut cursor = Cursor::new(Vec::new());

    let mut writer = WavWriter::new(&mut cursor, spec)
        .map_err(|e| format!("Failed to create WAV writer: {}", e))?;

    // Each 16-bit sample is 2 bytes (little-endian)
    for chunk in pcm_bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        writer
            .write_sample(sample)
            .map_err(|e| format!("Failed to write audio sample: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

    Ok(cursor.into_inner())
}

/// Writes raw PCM bytes to a WAV file path on disk.
pub fn write_pcm_to_wav_file(pcm_bytes: &[u8], target_path: &str) -> Result<(), String> {
    let spec = get_standard_wav_spec();
    let mut writer = WavWriter::create(target_path, spec)
        .map_err(|e| format!("Failed to create WAV file at {}: {}", target_path, e))?;

    for chunk in pcm_bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        writer
            .write_sample(sample)
            .map_err(|e| format!("Failed to write sample to file: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize WAV file at {}: {}", target_path, e))?;

    info!("WAV audio file written successfully to: {}", target_path);
    Ok(())
}

/// Generates PCM silence samples for a given duration in milliseconds.
pub fn generate_silence_pcm(duration_ms: u64) -> Vec<u8> {
    let num_samples = (DEFAULT_SAMPLE_RATE as u64 * duration_ms) / 1000;
    let byte_count = num_samples * 2; // 16-bit = 2 bytes per sample
    vec![0u8; byte_count as usize]
}

/// Reads duration in milliseconds from a WAV file path.
pub fn get_wav_duration_ms(wav_path: &str) -> Result<u64, String> {
    let reader = WavReader::open(wav_path)
        .map_err(|e| format!("Failed to open WAV file: {}", e))?;
    let spec = reader.spec();
    let num_samples = reader.duration() as u64;
    if spec.sample_rate == 0 {
        return Ok(0);
    }
    let duration_ms = (num_samples * 1000) / spec.sample_rate as u64;
    Ok(duration_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_to_wav_conversion() {
        // Create 200 samples of 0i16 silence (400 bytes)
        let dummy_pcm = vec![0u8; 400];
        let wav_bytes = pcm_to_wav_bytes(&dummy_pcm).expect("WAV conversion failed");

        // RIFF header is at least 44 bytes
        assert!(wav_bytes.len() > 44);
        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");

        // Validate via Hound WavReader
        let cursor = Cursor::new(wav_bytes);
        let reader = WavReader::new(cursor).expect("Should parse as valid WAV");
        assert_eq!(reader.spec().sample_rate, 24000);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().bits_per_sample, 16);
    }

    #[test]
    fn test_silence_generation() {
        let silence = generate_silence_pcm(1000); // 1 second
        // 24000 samples * 2 bytes = 48000 bytes
        assert_eq!(silence.len(), 48000);
        assert!(silence.iter().all(|&b| b == 0));
    }
}
