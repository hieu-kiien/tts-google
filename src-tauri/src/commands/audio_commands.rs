use crate::audio::wav_merger::merge_wav_files;
use crate::error::{AppError, AppResult};
use crate::models::segment::SegmentStatus;
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
) -> AppResult<MergeResult> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    let segments = ProjectRepository::get_segments_for_project(db, &project_id)
        .map_err(AppError::DatabaseError)?;

    let total_count = segments.len();
    let failed_count = segments
        .iter()
        .filter(|s| s.status == SegmentStatus::Failed)
        .count();
    let pending_count = segments
        .iter()
        .filter(|s| {
            s.status == SegmentStatus::Pending
                || s.status == SegmentStatus::Queued
                || s.status == SegmentStatus::Processing
        })
        .count();

    let valid_audio_paths: Vec<String> = segments
        .iter()
        .filter(|s| s.status == SegmentStatus::Success || s.status == SegmentStatus::Approved)
        .filter_map(|s| s.audio_path.clone())
        .collect();

    if valid_audio_paths.is_empty() {
        return Err(AppError::ValidationFailed(
            "Không có đoạn audio nào đã tạo thành công để ghép file".to_string(),
        ));
    }

    let warning = if failed_count > 0 || pending_count > 0 {
        Some(format!("Cảnh báo: {} đoạn lỗi, {} đoạn chưa xử lý trong tổng {} đoạn. File audio có thể bị thiếu nội dung.", failed_count, pending_count, total_count))
    } else {
        None
    };

    let proj =
        ProjectRepository::get_project_by_id(db, &project_id).map_err(AppError::DatabaseError)?;
    let proj_name = proj
        .map(|p| p.name)
        .unwrap_or_else(|| "audiobook".to_string());
    let safe_name = proj_name.replace(|c: char| !c.is_alphanumeric(), "_");

    let target_path = match custom_output_path {
        Some(path_str) if !path_str.trim().is_empty() => {
            resolve_write_target(&[state.output_dir.as_path()], &path_str, &["wav"])
                .map_err(AppError::FileSystem)?
        }
        _ => {
            let filename = format!("{}_master.wav", safe_name);
            let default_path = state.output_dir.join(filename);
            resolve_write_target(
                &[state.output_dir.as_path()],
                default_path.to_str().unwrap(),
                &["wav"],
            )
            .map_err(AppError::FileSystem)?
        }
    };

    let total_duration_ms = merge_wav_files(
        &valid_audio_paths,
        target_path.to_str().unwrap(),
        silence_gap_ms,
    )
    .map_err(AppError::AudioCorrupt)?;

    Ok(MergeResult {
        output_path: target_path.to_string_lossy().to_string(),
        total_duration_ms,
        warning,
    })
}

#[tauri::command]
pub fn export_project_subtitles(
    project_id: String,
    format_type: String, // "srt" | "vtt" | "lrc"
    silence_gap_ms: u64,
    custom_output_path: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<SubtitleExportResult> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    let segments = ProjectRepository::get_segments_for_project(db, &project_id)
        .map_err(AppError::DatabaseError)?;

    if segments.is_empty() {
        return Err(AppError::ValidationFailed(
            "Dự án không có segment nào để xuất phụ đề.".to_string(),
        ));
    }

    let fmt_lower = format_type.to_lowercase();
    let ext = match fmt_lower.as_str() {
        "vtt" => "vtt",
        "lrc" => "lrc",
        _ => "srt",
    };

    let content = match ext {
        "vtt" => generate_vtt_subtitles(&segments, silence_gap_ms),
        "lrc" => crate::text::lrc_exporter::generate_lrc_subtitles(&segments, silence_gap_ms),
        _ => generate_srt_subtitles(&segments, silence_gap_ms),
    };

    let proj =
        ProjectRepository::get_project_by_id(db, &project_id).map_err(AppError::DatabaseError)?;
    let proj_name = proj
        .map(|p| p.name)
        .unwrap_or_else(|| "subtitles".to_string());
    let safe_name = proj_name.replace(|c: char| !c.is_alphanumeric(), "_");

    let target_path = match custom_output_path {
        Some(path_str) if !path_str.trim().is_empty() => {
            resolve_export_target(&path_str, &[ext]).map_err(AppError::FileSystem)?
        }
        _ => {
            let filename = format!("{}.{}", safe_name, ext);
            let default_path = state.output_dir.join(filename);
            resolve_export_target(default_path.to_str().unwrap(), &[ext])
                .map_err(AppError::FileSystem)?
        }
    };

    fs::write(&target_path, &content)
        .map_err(|e| AppError::FileSystem(format!("Không thể ghi file phụ đề: {}", e)))?;

    Ok(SubtitleExportResult {
        output_path: target_path.to_string_lossy().to_string(),
        content,
    })
}

#[tauri::command]
pub fn export_audio_file(
    source_path: String,
    target_path: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let src = resolve_existing_read_target(
        &[state.app_data_dir.as_path(), state.output_dir.as_path()],
        &source_path,
        &["wav", "mp3", "m4a"],
    )
    .map_err(AppError::FileSystem)?;

    let dst = resolve_export_target(&target_path, &["wav", "mp3", "m4a"])
        .map_err(AppError::FileSystem)?;

    fs::copy(&src, &dst)
        .map_err(|e| AppError::FileSystem(format!("Lỗi khi copy file audio: {}", e)))?;

    Ok(dst.to_string_lossy().to_string())
}

#[tauri::command]
pub fn read_audio_as_base64(audio_path: String, state: State<'_, AppState>) -> AppResult<String> {
    let resolved_path = resolve_existing_read_target(
        &[state.app_data_dir.as_path(), state.output_dir.as_path()],
        &audio_path,
        &["wav", "mp3", "m4a"],
    )
    .map_err(AppError::FileSystem)?;

    let bytes = fs::read(&resolved_path)
        .map_err(|e| AppError::FileSystem(format!("Không thể đọc file audio: {}", e)))?;

    let base64_str = BASE64_STANDARD.encode(&bytes);
    validate_base64_payload_size(&base64_str, 50 * 1024 * 1024)
        .map_err(AppError::ValidationFailed)?;

    let mime = if audio_path.ends_with(".mp3") {
        "audio/mp3"
    } else if audio_path.ends_with(".m4a") {
        "audio/m4a"
    } else {
        "audio/wav"
    };

    Ok(format!("data:{};base64,{}", mime, base64_str))
}

#[tauri::command]
pub fn check_audio_file_exists(audio_path: String, state: State<'_, AppState>) -> AppResult<bool> {
    match resolve_existing_read_target(
        &[state.app_data_dir.as_path(), state.output_dir.as_path()],
        &audio_path,
        &["wav", "mp3", "m4a"],
    ) {
        Ok(path) => Ok(path.exists()),
        Err(_) => Ok(false),
    }
}
