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
    use crate::security::path_policy::{resolve_write_target, resolve_existing_read_target, validate_base64_payload_size};
    use crate::security::input_validation::{validate_project_name, validate_source_text, validate_api_key};
    use crate::models::registry::{validate_tts_model, MODEL_GEMINI_31_FLASH_TTS, MODEL_GEMINI_25_FLASH_TTS};
    use chrono::Utc;
    use std::path::Path;

    #[test]
    fn test_end_to_end_pipeline_simulation() {
        let raw_text = "# Chương 1: Giới thiệu\nGiá bản quyền là 500.000 VNĐ, phát hành ngày 15/08/2026 với ưu đãi 20%.";
        let normalized_text = VietnameseNormalizer::normalize(raw_text);

        assert!(normalized_text.contains("Việt Nam đồng"));
        assert!(normalized_text.contains("ngày 15 tháng 08 năm 2026"));
        assert!(normalized_text.contains("20 phần trăm"));

        let chunks = chunk_vietnamese_text(&normalized_text, None);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].index, 0);

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

        let dummy_pcm_1 = vec![0u8; 9600];
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

        let _ = std::fs::remove_file(wav_path1);
        let _ = std::fs::remove_file(wav_path2);
        let _ = std::fs::remove_file(merged_path);
    }

    #[test]
    fn test_path_policy_security() {
        let temp_dir = std::env::temp_dir();

        // 1. Disallow path traversal '..'
        let traversal_path = temp_dir.join("../outside.wav").to_string_lossy().to_string();
        let res = resolve_write_target(&[temp_dir.as_path()], &traversal_path, &["wav"]);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Path traversal"));

        // 2. Disallow unallowed extensions (.exe)
        let invalid_ext_path = temp_dir.join("malicious.exe").to_string_lossy().to_string();
        let res = resolve_write_target(&[temp_dir.as_path()], &invalid_ext_path, &["wav", "mp3"]);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not allowed"));

        // 3. Allow uppercase extension (.WAV)
        let uppercase_path = temp_dir.join("audio_test.WAV").to_string_lossy().to_string();
        let res = resolve_write_target(&[temp_dir.as_path()], &uppercase_path, &["wav"]);
        assert!(res.is_ok());

        // 4. Disallow path outside allowed roots
        let outside_dir = Path::new("C:/Windows/System32/config.txt");
        let res = resolve_existing_read_target(&[temp_dir.as_path()], outside_dir.to_str().unwrap(), &["txt"]);
        assert!(res.is_err());
    }

    #[test]
    fn test_input_validation_rules() {
        // Project name rules
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("   ").is_err());
        assert!(validate_project_name(&"a".repeat(300)).is_err());
        assert!(validate_project_name("Dự án TTS Tiếng Việt").is_ok());

        // Source text rules
        assert!(validate_source_text("").is_err());
        assert!(validate_source_text(&"x".repeat(600_000)).is_err());
        assert!(validate_source_text("Xin chào Việt Nam!").is_ok());

        // API Key rules
        assert!(validate_api_key("").is_err());
        assert!(validate_api_key("AIzaSyTestKeyWith Space").is_err());
        assert!(validate_api_key("AIzaSyValidGeminiKeyFormat123456").is_ok());
    }

    #[test]
    fn test_model_validation_and_fingerprints() {
        assert!(validate_tts_model(MODEL_GEMINI_31_FLASH_TTS).is_ok());
        assert!(validate_tts_model(MODEL_GEMINI_25_FLASH_TTS).is_ok());
        assert!(validate_tts_model("invalid-model-name").is_err());

        let input_31 = SegmentFingerprintInput {
            text: "Hello",
            voice: "Kore",
            model: MODEL_GEMINI_31_FLASH_TTS,
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
            sample_rate_hz: 24000,
        };
        let input_25 = SegmentFingerprintInput {
            text: "Hello",
            voice: "Kore",
            model: MODEL_GEMINI_25_FLASH_TTS,
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
            sample_rate_hz: 24000,
        };

        let fp_31 = compute_segment_fingerprint(&input_31);
        let fp_25 = compute_segment_fingerprint(&input_25);

        assert_ne!(fp_31, fp_25);
    }

    #[test]
    fn test_base64_pre_decode_limit() {
        // Enforce 100-byte limit on base64 input
        let small_base64 = "SGVsbG8gV29ybGQ="; // "Hello World"
        let res = validate_base64_payload_size(small_base64, 1024);
        assert!(res.is_ok());

        let huge_base64 = "A".repeat(5000);
        let res = validate_base64_payload_size(&huge_base64, 100);
        assert!(res.is_err());
    }
}
