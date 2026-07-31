#[cfg(test)]
mod tests {
    use crate::audio::pcm_wav::{pcm_to_wav_bytes, write_pcm_to_wav_file, get_wav_duration_ms};
    use crate::audio::wav_merger::merge_wav_files;
    use crate::storage::db::DatabaseManager;
    use crate::storage::project_repo::{ProjectRepository, ProjectRecord, SegmentRecord};
    use crate::text::chunker::chunk_vietnamese_text;
    use crate::text::fingerprint::{compute_segment_fingerprint, SegmentFingerprintInput};
    use crate::text::normalizer::VietnameseNormalizer;
    use crate::text::srt_exporter::generate_srt_subtitles;
    use crate::text::vtt_exporter::generate_vtt_subtitles;
    use chrono::Utc;

    #[test]
    fn test_end_to_end_pipeline_simulation() {
        // 1. Text Normalization
        let raw_text = "# Chương 1: Giới thiệu\nGiá bản quyền là 500.000 VNĐ, phát hành ngày 15/08/2026 với ưu đãi 20%.";
        let normalized_text = VietnameseNormalizer::normalize(raw_text);

        assert!(normalized_text.contains("Việt Nam đồng"));
        assert!(normalized_text.contains("ngày 15 tháng 08 năm 2026"));
        assert!(normalized_text.contains("20 phần trăm"));

        // 2. Text Chunking
        let chunks = chunk_vietnamese_text(&normalized_text, None);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].index, 0);

        // 3. Fingerprint computation
        let fp_input = SegmentFingerprintInput {
            text: &chunks[0].text,
            voice: "Kore",
            model: "gemini-3.1-flash-tts-preview",
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
            sample_rate_hz: 24000,
        };
        let fp1 = compute_segment_fingerprint(&fp_input);
        let fp2 = compute_segment_fingerprint(&fp_input);
        assert_eq!(fp1, fp2);

        // 4. DB Persistence
        let db = DatabaseManager::in_memory().expect("Failed to init in-memory database");

        let now_str = Utc::now().to_rfc3339();
        let proj = ProjectRecord {
            id: "proj_e2e_001".to_string(),
            name: "E2E Test Project".to_string(),
            source_text: raw_text.to_string(),
            model: "gemini-3.1-flash-tts-preview".to_string(),
            voice: "Kore".to_string(),
            preset: "default".to_string(),
            pacing: "normal".to_string(),
            pronunciation_notes: None,
            output_directory: "D:/output".to_string(),
            status: "draft".to_string(),
            created_at: now_str.clone(),
            updated_at: now_str,
        };

        ProjectRepository::create_project(&db, &proj)
            .expect("Failed to create project record in DB");

        let projects = ProjectRepository::list_projects(&db)
            .expect("Failed to query projects from DB");

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "E2E Test Project");

        // 5. Audio Generation & Silence Padding
        let dummy_pcm_1 = vec![0u8; 9600]; // 0.2s silence
        let wav_bytes_1 = pcm_to_wav_bytes(&dummy_pcm_1).expect("Failed PCM->WAV conversion");
        assert!(wav_bytes_1.len() > 44);

        let temp_dir = std::env::temp_dir();
        let test_uid = uuid::Uuid::new_v4().to_string();
        let wav_path1 = temp_dir.join(format!("test_seg_1_{}.wav", test_uid));
        let wav_path2 = temp_dir.join(format!("test_seg_2_{}.wav", test_uid));
        let merged_path = temp_dir.join(format!("test_merged_{}.wav", test_uid));

        write_pcm_to_wav_file(&dummy_pcm_1, wav_path1.to_str().unwrap())
            .expect("Failed to write WAV 1");
        write_pcm_to_wav_file(&dummy_pcm_1, wav_path2.to_str().unwrap())
            .expect("Failed to write WAV 2");

        // 6. WAV Merging
        let files_to_merge = vec![
            wav_path1.to_str().unwrap().to_string(),
            wav_path2.to_str().unwrap().to_string(),
        ];
        let total_duration = merge_wav_files(&files_to_merge, merged_path.to_str().unwrap(), 300)
            .expect("WAV merging failed");

        assert!(total_duration > 0);

        let actual_dur = get_wav_duration_ms(merged_path.to_str().unwrap())
            .expect("Failed to read merged duration");

        assert!(actual_dur > 0);

        // 7. Subtitle Exporting
        let seg_record = SegmentRecord {
            id: "seg_001".to_string(),
            project_id: proj.id.clone(),
            position: 1,
            text: "Chương 1: Giới thiệu".to_string(),
            prompt: "Chương 1: Giới thiệu".to_string(),
            status: "completed".to_string(),
            attempts: 1,
            audio_path: Some(wav_path1.to_str().unwrap().to_string()),
            duration_ms: 2000,
            error_code: None,
            error_message: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            fingerprint: Some(fp1),
            output_fingerprint: None,
            attempt_count: 1,
            next_retry_at: None,
            queued_at: None,
            started_at: None,
            finished_at: None,
            lease_owner: None,
            lease_expires_at: None,
            last_error_code: None,
            last_error_message: None,
            cancel_requested: false,
            state_revision: 1,
            output_size: 9600,
            voice: None,
        };

        let segments = vec![seg_record];
        let srt_content = generate_srt_subtitles(&segments, 300);
        let vtt_content = generate_vtt_subtitles(&segments, 300);

        assert!(srt_content.contains("Chương 1: Giới thiệu"));
        assert!(vtt_content.contains("WEBVTT"));

        // Cleanup temporary files
        let _ = std::fs::remove_file(wav_path1);
        let _ = std::fs::remove_file(wav_path2);
        let _ = std::fs::remove_file(merged_path);
    }
}
