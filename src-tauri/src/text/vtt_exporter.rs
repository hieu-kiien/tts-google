use crate::storage::project_repo::SegmentRecord;

/// Formats milliseconds into WebVTT timestamp format: HH:MM:SS.mmm
pub fn format_vtt_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let rem_h = ms % 3_600_000;
    let minutes = rem_h / 60_000;
    let rem_m = rem_h % 60_000;
    let seconds = rem_m / 1_000;
    let millis = rem_m % 1_000;

    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

/// Generates a complete WebVTT (.vtt) subtitle string from a list of project text segments.
pub fn generate_vtt_subtitles(segments: &[SegmentRecord], silence_gap_ms: u64) -> String {
    let mut vtt_out = String::from("WEBVTT - Auto TTS Desktop Studio Generated\n\n");
    let mut current_time_ms: u64 = 0;

    for (idx, seg) in segments.iter().enumerate() {
        let duration = if seg.duration_ms > 0 {
            seg.duration_ms
        } else {
            ((seg.text.chars().count() as f64 / 15.0) * 1000.0) as u64
        };

        let start_ts = format_vtt_timestamp(current_time_ms);
        let end_time_ms = current_time_ms + duration;
        let end_ts = format_vtt_timestamp(end_time_ms);

        vtt_out.push_str(&format!("{}\n", idx + 1));
        vtt_out.push_str(&format!("{} --> {}\n", start_ts, end_ts));
        vtt_out.push_str(&format!("{}\n\n", seg.text.trim()));

        current_time_ms = end_time_ms + silence_gap_ms;
    }

    vtt_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_vtt_timestamp() {
        assert_eq!(format_vtt_timestamp(0), "00:00:00.000");
        assert_eq!(format_vtt_timestamp(1500), "00:00:01.500");
    }
}
