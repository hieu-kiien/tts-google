use crate::storage::project_repo::SegmentRecord;
use std::fs;
use std::path::Path;
use tracing::info;

pub struct LrcSegment {
    pub start_ms: u64,
    pub text: String,
}

pub struct LrcExporter;

impl LrcExporter {
    pub fn export_lrc<P: AsRef<Path>>(
        segments: &[LrcSegment],
        title: &str,
        artist: &str,
        output_path: P,
    ) -> Result<String, String> {
        let mut content = String::new();
        content.push_str(&format!("[ti:{}]\n", title));
        content.push_str(&format!("[ar:{}]\n", artist));
        content.push_str("[by:Auto TTS Desktop Studio]\n");
        content.push_str("[re:Gemini TTS Free Tier]\n\n");

        for seg in segments {
            let total_secs = seg.start_ms / 1000;
            let minutes = total_secs / 60;
            let seconds = total_secs % 60;
            let hundredths = (seg.start_ms % 1000) / 10;

            let timestamp = format!("[{:02}:{:02}.{:02}]", minutes, seconds, hundredths);
            content.push_str(&format!("{} {}\n", timestamp, seg.text.trim()));
        }

        let path_ref = output_path.as_ref();
        fs::write(path_ref, content.as_bytes()).map_err(|e| format!("Lỗi ghi file LRC: {}", e))?;

        info!("Exported LRC subtitles to {:?}", path_ref);
        Ok(path_ref.to_string_lossy().to_string())
    }
}

pub fn generate_lrc_subtitles(segments: &[SegmentRecord], silence_gap_ms: u64) -> String {
    let mut content = String::new();
    content.push_str("[by:Auto TTS Desktop]\n\n");

    let mut current_ms: u64 = 0;
    for seg in segments {
        let text = seg.text.trim();
        if text.is_empty() {
            continue;
        }

        let total_secs = current_ms / 1000;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        let hundredths = (current_ms % 1000) / 10;

        content.push_str(&format!(
            "[{:02}:{:02}.{:02}] {}\n",
            minutes, seconds, hundredths, text
        ));
        let duration = if seg.duration_ms > 0 {
            seg.duration_ms
        } else {
            3000
        };
        current_ms += duration + silence_gap_ms;
    }

    content
}
