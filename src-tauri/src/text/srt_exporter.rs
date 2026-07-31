use crate::storage::project_repo::SegmentRecord;

/// Formats milliseconds into SubRip timestamp format: HH:MM:SS,mmm
pub fn format_srt_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let rem_h = ms % 3_600_000;
    let minutes = rem_h / 60_000;
    let rem_m = rem_h % 60_000;
    let seconds = rem_m / 1_000;
    let millis = rem_m % 1_000;

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

/// Generates a complete SubRip (.srt) subtitle string from a list of project text segments.
pub fn generate_srt_subtitles(segments: &[SegmentRecord], silence_gap_ms: u64) -> String {
    let mut srt_out = String::new();
    let mut current_time_ms: u64 = 0;

    for (idx, seg) in segments.iter().enumerate() {
        let duration = if seg.duration_ms > 0 {
            seg.duration_ms
        } else {
            // Estimate based on char length if audio not yet generated (~15 chars/sec)
            ((seg.text.chars().count() as f64 / 15.0) * 1000.0) as u64
        };

        let start_ts = format_srt_timestamp(current_time_ms);
        let end_time_ms = current_time_ms + duration;
        let end_ts = format_srt_timestamp(end_time_ms);

        srt_out.push_str(&format!("{}\n", idx + 1));
        srt_out.push_str(&format!("{} --> {}\n", start_ts, end_ts));
        srt_out.push_str(&format!("{}\n\n", seg.text.trim()));

        current_time_ms = end_time_ms + silence_gap_ms;
    }

    srt_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_srt_timestamp() {
        assert_eq!(format_srt_timestamp(0), "00:00:00,000");
        assert_eq!(format_srt_timestamp(1500), "00:00:01,500");
        assert_eq!(format_srt_timestamp(3661005), "01:01:01,005");
    }
}
