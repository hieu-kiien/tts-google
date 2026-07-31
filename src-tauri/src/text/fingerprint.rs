use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct SegmentFingerprintInput<'a> {
    pub text: &'a str,
    pub voice: &'a str,
    pub model: &'a str,
    pub speaking_rate: f32,
    pub pitch_shift: f32,
    pub volume_gain_db: f32,
    pub sample_rate_hz: u32,
}

pub fn compute_segment_fingerprint(input: &SegmentFingerprintInput) -> String {
    let canonical_str = format!(
        "tts-fingerprint:v1\ntext={}\nvoice={}\nmodel={}\nrate={:.2}\npitch={:.2}\nvol={:.2}\nsample_rate={}\n",
        input.text.trim(),
        input.voice.trim(),
        input.model.trim(),
        input.speaking_rate,
        input.pitch_shift,
        input.volume_gain_db,
        input.sample_rate_hz
    );

    let mut hasher = Sha256::new();
    hasher.update(canonical_str.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_fingerprint_deterministic() {
        let input1 = SegmentFingerprintInput {
            text: "Xin chào Việt Nam",
            voice: "Kore",
            model: "gemini-3.1-flash-tts-preview",
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
            sample_rate_hz: 24000,
        };

        let input2 = SegmentFingerprintInput {
            text: "Xin chào Việt Nam",
            voice: "Kore",
            model: "gemini-3.1-flash-tts-preview",
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
            sample_rate_hz: 24000,
        };

        assert_eq!(compute_segment_fingerprint(&input1), compute_segment_fingerprint(&input2));
    }

    #[test]
    fn test_compute_fingerprint_changes_on_voice_or_text() {
        let input1 = SegmentFingerprintInput {
            text: "Xin chào",
            voice: "Kore",
            model: "gemini-3.1-flash-tts-preview",
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
            sample_rate_hz: 24000,
        };

        let input2 = SegmentFingerprintInput {
            text: "Xin chào",
            voice: "Puck",
            model: "gemini-3.1-flash-tts-preview",
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
            sample_rate_hz: 24000,
        };

        assert_ne!(compute_segment_fingerprint(&input1), compute_segment_fingerprint(&input2));
    }
}
