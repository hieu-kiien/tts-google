use crate::audio::wav_merger::merge_wav_files;
use crate::security::path_policy::{
    resolve_existing_read_target, resolve_export_target, resolve_write_target,
    validate_base64_payload_size,
};
use crate::state::app_state::AppState;
use crate::storage::project_repo::ProjectRepository;
use crate::text::srt_exporter::generate_srt_subtitles;
use crate::text::vtt_exporter::generate_vtt_subtitles;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use std::fs;
use tauri::State;

#[derive(serde::Serialize)]
pub struct MergeResult {
    pub output_path: String,
    pub total_duration_ms: u64,
    pub warning: Option<String>,
}

#[derive(serde::Serialize)]
pub struct SubtitleExportResult {
    pub output_path: String,
    pub content: String,
}

#[tauri::command]
pub fn merge_project_audio(
    project_id: String,
    silence_gap_ms: u64,
    custom_output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<MergeResult, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    let segments = ProjectRepository::get_segments_for_project(db, &project_id)?;

    let total_count = segments.len();
    let failed_count = segments.iter().filter(|s| s.status == "failed").count();
    let pending_count = segments
        .iter()
        .filter(|s| s.status == "pending" || s.status == "queued" || s.status == "processing")
        .count();

    let valid_audio_paths: Vec<String> = segments
        .iter()
        .filter(|s| s.status == "success")
        .filter_map(|s| s.audio_path.clone())
        .collect();

    if valid_audio_paths.is_empty() {
        return Err("Không có đoạn audio nào đã tạo thành công để ghép file".to_string());
    }

    let warning = if failed_count > 0 || pending_count > 0 {
        Some(format!("Cảnh báo: {} đoạn lỗi, {} đoạn chưa xử lý trong tổng {} đoạn. File audio có thể bị thiếu nội dung.", failed_count, pending_count, total_count))
    } else {
        None
    };

    let is_user_path = custom_output_path.as_ref().map_or(false, |p| !p.trim().is_empty());
    let raw_master_path = match custom_output_path {
        Some(p) if !p.trim().is_empty() => p,
        _ => state
            .output_dir
            .join(format!("master_{}.wav", project_id))
            .to_string_lossy()
            .to_string(),
    };

    // User-selected paths (from save dialog) use relaxed export policy;
    // Default app paths use strict write policy with allowed_roots
    let master_path = if is_user_path {
        resolve_export_target(&raw_master_path, &["wav"])?
    } else {
        resolve_write_target(&state.get_allowed_roots(), &raw_master_path, &["wav"])?
    };
    let master_path_str = master_path.to_string_lossy().to_string();

    let total_duration_ms = merge_wav_files(&valid_audio_paths, &master_path_str, silence_gap_ms)?;

    Ok(MergeResult {
        output_path: master_path_str,
        total_duration_ms,
        warning,
    })
}

#[tauri::command]
pub fn export_project_srt(
    project_id: String,
    silence_gap_ms: u64,
    custom_output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<SubtitleExportResult, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    let segments = ProjectRepository::get_segments_for_project(db, &project_id)?;

    if segments.is_empty() {
        return Err("Dự án không có segment nào để xuất phụ đề".to_string());
    }

    let srt_content = generate_srt_subtitles(&segments, silence_gap_ms);

    let is_user_path = custom_output_path.as_ref().map_or(false, |p| !p.trim().is_empty());
    let raw_output_path = match custom_output_path {
        Some(p) if !p.trim().is_empty() => p,
        _ => state
            .output_dir
            .join(format!("subtitle_{}.srt", project_id))
            .to_string_lossy()
            .to_string(),
    };

    let output_path = if is_user_path {
        resolve_export_target(&raw_output_path, &["srt"])?
    } else {
        resolve_write_target(&state.get_allowed_roots(), &raw_output_path, &["srt"])?
    };
    let output_path_str = output_path.to_string_lossy().to_string();

    fs::write(&output_path, &srt_content).map_err(|e| {
        format!(
            "Không thể ghi file phụ đề SRT tại {}: {}",
            output_path_str, e
        )
    })?;

    Ok(SubtitleExportResult {
        output_path: output_path_str,
        content: srt_content,
    })
}

#[tauri::command]
pub fn export_project_vtt(
    project_id: String,
    silence_gap_ms: u64,
    custom_output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<SubtitleExportResult, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    let segments = ProjectRepository::get_segments_for_project(db, &project_id)?;

    if segments.is_empty() {
        return Err("Dự án không có segment nào để xuất phụ đề VTT".to_string());
    }

    let vtt_content = generate_vtt_subtitles(&segments, silence_gap_ms);

    let is_user_path = custom_output_path.as_ref().map_or(false, |p| !p.trim().is_empty());
    let raw_output_path = match custom_output_path {
        Some(p) if !p.trim().is_empty() => p,
        _ => state
            .output_dir
            .join(format!("subtitle_{}.vtt", project_id))
            .to_string_lossy()
            .to_string(),
    };

    let output_path = if is_user_path {
        resolve_export_target(&raw_output_path, &["vtt"])?
    } else {
        resolve_write_target(&state.get_allowed_roots(), &raw_output_path, &["vtt"])?
    };
    let output_path_str = output_path.to_string_lossy().to_string();

    fs::write(&output_path, &vtt_content).map_err(|e| {
        format!(
            "Không thể ghi file phụ đề VTT tại {}: {}",
            output_path_str, e
        )
    })?;

    Ok(SubtitleExportResult {
        output_path: output_path_str,
        content: vtt_content,
    })
}

#[tauri::command]
pub fn export_project_lrc(
    project_id: String,
    silence_gap_ms: u64,
    custom_output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<SubtitleExportResult, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    let segments = ProjectRepository::get_segments_for_project(db, &project_id)?;

    if segments.is_empty() {
        return Err("Dự án không có segment nào để xuất phụ đề LRC".to_string());
    }

    let mut current_time = 0u64;
    let lrc_segs: Vec<crate::text::lrc_exporter::LrcSegment> = segments
        .iter()
        .map(|s| {
            let seg = crate::text::lrc_exporter::LrcSegment {
                start_ms: current_time,
                text: s.text.clone(),
            };
            current_time += s.duration_ms + silence_gap_ms;
            seg
        })
        .collect();

    let is_user_path = custom_output_path.as_ref().map_or(false, |p| !p.trim().is_empty());
    let raw_output_path = match custom_output_path {
        Some(p) if !p.trim().is_empty() => p,
        _ => state
            .output_dir
            .join(format!("subtitle_{}.lrc", project_id))
            .to_string_lossy()
            .to_string(),
    };

    let output_path = if is_user_path {
        resolve_export_target(&raw_output_path, &["lrc"])?
    } else {
        resolve_write_target(&state.get_allowed_roots(), &raw_output_path, &["lrc"])?
    };
    let output_path_str = output_path.to_string_lossy().to_string();

    let exported_path = crate::text::lrc_exporter::LrcExporter::export_lrc(
        &lrc_segs,
        "Auto TTS Project",
        "Gemini TTS Reader",
        &output_path_str,
    )?;

    let content = fs::read_to_string(&exported_path).unwrap_or_default();

    Ok(SubtitleExportResult {
        output_path: exported_path,
        content,
    })
}

#[tauri::command]
pub fn export_project_m4b_manifest(
    project_id: String,
    silence_gap_ms: u64,
    custom_output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<SubtitleExportResult, String> {
    let db = state.db.as_ref().ok_or("Database not initialized")?;
    let segments = ProjectRepository::get_segments_for_project(db, &project_id)?;

    if segments.is_empty() {
        return Err("Dự án không có segment nào để xuất Audiobook Manifest".to_string());
    }

    let mut current_time = 0u64;
    let chapters: Vec<crate::audio::m4b_exporter::ChapterMarker> = segments
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            let marker = crate::audio::m4b_exporter::ChapterMarker {
                chapter_number: idx + 1,
                title: format!("Segment #{}", s.position),
                start_time_ms: current_time,
                duration_ms: s.duration_ms,
            };
            current_time += s.duration_ms + silence_gap_ms;
            marker
        })
        .collect();

    let manifest = crate::audio::m4b_exporter::AudiobookManifest {
        title: format!("Audiobook Project {}", project_id),
        author: "Tác giả".to_string(),
        narrator: "Gemini Free Tier Studio".to_string(),
        publisher: "Auto TTS Desktop".to_string(),
        total_duration_ms: current_time,
        chapters,
    };

    let is_user_path = custom_output_path.as_ref().map_or(false, |p| !p.trim().is_empty());
    let raw_output_path = match custom_output_path {
        Some(p) if !p.trim().is_empty() => p,
        _ => state
            .output_dir
            .join(format!("audiobook_{}.m4b.json", project_id))
            .to_string_lossy()
            .to_string(),
    };

    let output_path = if is_user_path {
        resolve_export_target(&raw_output_path, &["json"])?
    } else {
        resolve_write_target(&state.get_allowed_roots(), &raw_output_path, &["json"])?
    };
    let output_path_str = output_path.to_string_lossy().to_string();

    let exported_path =
        crate::audio::m4b_exporter::M4bExporter::export_manifest(&manifest, &output_path_str)?;

    let content = fs::read_to_string(&exported_path).unwrap_or_default();

    Ok(SubtitleExportResult {
        output_path: exported_path,
        content,
    })
}

#[tauri::command]
pub fn save_single_segment_audio(
    source_audio_path: String,
    target_output_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let source_path =
        resolve_existing_read_target(&state.get_allowed_roots(), &source_audio_path, &["wav"])?;
    let target_path =
        resolve_write_target(&state.get_allowed_roots(), &target_output_path, &["wav"])?;

    fs::copy(&source_path, &target_path)
        .map_err(|e| format!("Không thể lưu file audio segment: {}", e))?;

    Ok(target_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn read_audio_data_url(
    path: Option<String>,
    file_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let target = path
        .or(file_path)
        .ok_or_else(|| "Chưa cung cấp đường dẫn file audio".to_string())?;

    if target.starts_with("data:") {
        return Ok(target);
    }

    let canonical_target = resolve_existing_read_target(
        &state.get_allowed_roots(),
        &target,
        &["wav", "mp3", "ogg", "flac"],
    )?;

    let bytes = fs::read(&canonical_target).map_err(|e| {
        format!(
            "Failed to read audio file at {}: {}",
            canonical_target.display(),
            e
        )
    })?;
    let base64_str = BASE64_STANDARD.encode(&bytes);
    Ok(format!("data:audio/wav;base64,{}", base64_str))
}

#[tauri::command]
pub fn write_binary_file(
    target_path: String,
    base64_data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let resolved_path = resolve_write_target(
        &state.get_allowed_roots(),
        &target_path,
        &["wav", "mp3", "txt", "json", "srt", "vtt", "lrc"],
    )?;

    // Max 50MB decoded limit
    let bytes = validate_base64_payload_size(&base64_data, 50 * 1024 * 1024)?;

    fs::write(&resolved_path, &bytes)
        .map_err(|e| format!("Không thể ghi tệp tại {}: {}", resolved_path.display(), e))?;

    Ok(resolved_path.to_string_lossy().to_string())
}
