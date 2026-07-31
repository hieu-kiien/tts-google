use hound::{WavReader, WavWriter};
use std::path::Path;
use std::fs;
use tracing::info;
use crate::audio::pcm_wav::{get_standard_wav_spec, generate_silence_pcm};


/// Merges multiple 24kHz 16-bit Mono WAV segment files into a single destination master WAV file safely with atomic rename.
pub fn merge_wav_files(
    segment_paths: &[String],
    output_path: &str,
    silence_gap_ms: u64,
) -> Result<u64, String> {
    if segment_paths.is_empty() {
        return Err("No segment files provided for merging".to_string());
    }

    let target_path = Path::new(output_path);
    let parent_dir = target_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    if !parent_dir.exists() {
        fs::create_dir_all(parent_dir)
            .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    }

    // Atomic temp file in SAME directory to ensure cross-device rename works on Windows
    let temp_filename = format!(
        ".tmp_master_{}.wav",
        uuid::Uuid::new_v4().simple()
    );
    let temp_output_path = parent_dir.join(temp_filename);
    let temp_output_str = temp_output_path.to_str().ok_or("Invalid output path")?;

    let spec = get_standard_wav_spec();
    let mut writer = WavWriter::create(temp_output_str, spec)
        .map_err(|e| format!("Failed to create temp master WAV file at {}: {}", temp_output_str, e))?;

    let silence_pcm = generate_silence_pcm(silence_gap_ms);
    let mut total_samples_written: u64 = 0;

    for (idx, seg_path) in segment_paths.iter().enumerate() {
        if !Path::new(seg_path).exists() {
            let _ = fs::remove_file(&temp_output_path);
            return Err(format!("Segment file does not exist: {}", seg_path));
        }

        let mut reader = WavReader::open(seg_path)
            .map_err(|e| {
                let _ = fs::remove_file(&temp_output_path);
                format!("Failed to open segment WAV at {}: {}", seg_path, e)
            })?;

        // Audio Spec Validation
        let r_spec = reader.spec();
        if r_spec.sample_rate != spec.sample_rate || r_spec.channels != spec.channels || r_spec.bits_per_sample != spec.bits_per_sample {
            let _ = fs::remove_file(&temp_output_path);
            return Err(format!(
                "Incompatible WAV spec in {}: expected {}Hz/{}ch/{}bit, got {}Hz/{}ch/{}bit",
                seg_path, spec.sample_rate, spec.channels, spec.bits_per_sample, r_spec.sample_rate, r_spec.channels, r_spec.bits_per_sample
            ));
        }

        // Read and normalize samples for peak volume consistency
        let raw_samples: Vec<i16> = reader.samples::<i16>()
            .collect::<Result<Vec<i16>, _>>()
            .map_err(|e| {
                let _ = fs::remove_file(&temp_output_path);
                format!("Corrupt sample in {}: {}", seg_path, e)
            })?;

        let max_peak = raw_samples.iter().map(|s| s.saturating_abs() as u32).max().unwrap_or(0);
        let scale = if max_peak > 1000 && max_peak < 32000 {
            28000.0 / max_peak as f32
        } else {
            1.0
        };

        for sample in raw_samples {
            let normalized_sample = if scale != 1.0 {
                ((sample as f32) * scale).clamp(-32768.0, 32767.0) as i16
            } else {
                sample
            };

            writer.write_sample(normalized_sample).map_err(|e| {
                let _ = fs::remove_file(&temp_output_path);
                format!("Failed to write sample: {}", e)
            })?;
            total_samples_written += 1;
        }

        // Insert silence gap between segments (not after the last segment)
        if idx < segment_paths.len() - 1 && silence_gap_ms > 0 {
            for chunk in silence_pcm.chunks_exact(2) {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                writer.write_sample(sample).map_err(|e| {
                    let _ = fs::remove_file(&temp_output_path);
                    format!("Failed to write silence: {}", e)
                })?;
                total_samples_written += 1;
            }
        }
    }

    writer
        .finalize()
        .map_err(|e| {
            let _ = fs::remove_file(&temp_output_path);
            format!("Failed to finalize master WAV file: {}", e)
        })?;

    // Atomic rename to target output path
    if let Err(rename_err) = fs::rename(&temp_output_path, target_path) {
        // Fallback: copy + remove for Windows (file may be locked by antivirus)
        info!("Atomic rename failed ({}), falling back to copy + remove.", rename_err);
        if let Err(copy_err) = fs::copy(&temp_output_path, target_path) {
            let _ = fs::remove_file(&temp_output_path);
            return Err(format!("Failed to copy master WAV: {}", copy_err));
        }
        let _ = fs::remove_file(&temp_output_path);
    }

    let total_duration_ms = (total_samples_written * 1000) / (spec.sample_rate as u64);
    info!(
        "Master WAV created atomically at {}. Total segments: {}, Duration: {}ms",
        output_path,
        segment_paths.len(),
        total_duration_ms
    );

    Ok(total_duration_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::audio::pcm_wav::write_pcm_to_wav_file;

    #[test]
    fn test_merge_wav_files_atomic() {
        let temp_dir = std::env::temp_dir();
        let path1 = temp_dir.join("seg1_atomic.wav").to_str().unwrap().to_string();
        let path2 = temp_dir.join("seg2_atomic.wav").to_str().unwrap().to_string();
        let out_path = temp_dir.join("master_out_atomic.wav").to_str().unwrap().to_string();

        let pcm1 = vec![0u8; 4800]; // 100ms 24kHz mono PCM
        let pcm2 = vec![0u8; 4800]; // 100ms 24kHz mono PCM

        write_pcm_to_wav_file(&pcm1, &path1).expect("Failed to write seg1");
        write_pcm_to_wav_file(&pcm2, &path2).expect("Failed to write seg2");

        let duration_ms = merge_wav_files(&[path1.clone(), path2.clone()], &out_path, 200)
            .expect("Failed to merge WAV files");

        assert!(duration_ms >= 400); // 100ms + 200ms silence + 100ms = 400ms

        let reader = WavReader::open(&out_path).expect("Master WAV should be valid");
        assert_eq!(reader.spec().sample_rate, 24000);
        assert_eq!(reader.spec().channels, 1);

        let _ = fs::remove_file(&path1);
        let _ = fs::remove_file(&path2);
        let _ = fs::remove_file(&out_path);
    }
}
