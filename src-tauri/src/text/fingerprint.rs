use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SegmentFingerprintV2 {
    pub schema_version: u8,
    pub provider_id: String,
    pub model_id: String,
    pub final_spoken_text: String,
    pub final_prompt: String,
    pub voice_id: String,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub sample_bits: u8,
    pub normalizer_version: String,
    pub prompt_builder_version: String,
}

impl Default for SegmentFingerprintV2 {
    fn default() -> Self {
        Self {
            schema_version: 2,
            provider_id: "google_gemini".to_string(),
            model_id: "gemini-3.1-flash-tts-preview".to_string(),
            final_spoken_text: String::new(),
            final_prompt: String::new(),
            voice_id: "Kore".to_string(),
            sample_rate_hz: 24000,
            channels: 1,
            sample_bits: 16,
            normalizer_version: "1.0.0".to_string(),
            prompt_builder_version: "1.0.0".to_string(),
        }
    }
}

pub fn compute_segment_fingerprint_v2(input: &SegmentFingerprintV2) -> String {
    let json_bytes = serde_json::to_vec(input).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&json_bytes);
    format!("fp2_{:x}", hasher.finalize())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyCacheClassification {
    Preserved,
    Reusable,
    Unverified,
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

        assert_eq!(
            compute_segment_fingerprint(&input1),
            compute_segment_fingerprint(&input2)
        );
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

        assert_ne!(
            compute_segment_fingerprint(&input1),
            compute_segment_fingerprint(&input2)
        );
    }

    #[test]
    fn test_compute_fingerprint_v2_canonical() {
        let fp_input1 = SegmentFingerprintV2 {
            final_spoken_text: "Nội dung chuẩn hóa".to_string(),
            voice_id: "Kore".to_string(),
            ..Default::default()
        };

        let fp_input2 = SegmentFingerprintV2 {
            final_spoken_text: "Nội dung chuẩn hóa".to_string(),
            voice_id: "Kore".to_string(),
            ..Default::default()
        };

        let fp1 = compute_segment_fingerprint_v2(&fp_input1);
        let fp2 = compute_segment_fingerprint_v2(&fp_input2);

        assert!(fp1.starts_with("fp2_"));
        assert_eq!(fp1, fp2);

        let mut fp_input3 = fp_input1.clone();
        fp_input3.voice_id = "Puck".to_string();
        let fp3 = compute_segment_fingerprint_v2(&fp_input3);

        assert_ne!(fp1, fp3);
    }
}
