use crate::error::{AppError, AppResult};
use crate::models::registry::{validate_tts_model, MODEL_GEMINI_31_FLASH_TTS};
use crate::models::segment::{ReviewStatus, SegmentStatus, SynthesisStatus};
use crate::security::input_validation::{
    validate_chunk_mode, validate_preset, validate_project_name, validate_source_text,
    validate_voice,
};
use crate::state::app_state::AppState;
use crate::storage::project_repo::{ProjectRecord, ProjectRepository, SegmentRecord};
use crate::storage::segment_repo::SegmentRepository;
use crate::text::chunker::chunk_vietnamese_text_by_mode;
use crate::text::fingerprint::{compute_segment_fingerprint, SegmentFingerprintInput};
use crate::text::prompt_builder::{build_tts_prompt, PromptStyleOptions};
use chrono::Utc;
use tauri::{Manager, State};
use uuid::Uuid;

#[tauri::command]
pub fn create_project(
    name: String,
    source_text: String,
    voice: String,
    preset: String,
    chunk_mode: Option<String>,
    model: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<ProjectRecord> {
    validate_project_name(&name).map_err(AppError::ValidationFailed)?;
    validate_source_text(&source_text).map_err(AppError::ValidationFailed)?;
    validate_voice(&voice).map_err(AppError::ValidationFailed)?;
    validate_preset(&preset).map_err(AppError::ValidationFailed)?;
    validate_chunk_mode(chunk_mode.as_deref()).map_err(AppError::ValidationFailed)?;

    let selected_model = match model {
        Some(m) if !m.trim().is_empty() => {
            validate_tts_model(&m).map_err(AppError::ValidationFailed)?;
            m
        }
        _ => MODEL_GEMINI_31_FLASH_TTS.to_string(),
    };

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    let proj_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let proj = ProjectRecord {
        id: proj_id.clone(),
        name,
        source_text: source_text.clone(),
        model: selected_model.clone(),
        voice: voice.clone(),
        preset: preset.clone(),
        pacing: "Bình thường".to_string(),
        pronunciation_notes: None,
        output_directory: state.output_dir.to_string_lossy().to_string(),
        status: "draft".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    ProjectRepository::create_project(db, &proj).map_err(AppError::DatabaseError)?;

    let mode_str = chunk_mode.as_deref().unwrap_or("auto");
    let chunks = chunk_vietnamese_text_by_mode(&source_text, mode_str);

    let prompt_opts = PromptStyleOptions {
        style_preset: preset,
        pacing: "Bình thường".to_string(),
        pronunciation_notes: None,
    };

    let segments: Vec<SegmentRecord> = chunks
        .into_iter()
        .map(|c| {
            let fp = compute_segment_fingerprint(&SegmentFingerprintInput {
                text: &c.text,
                voice: &voice,
                model: &selected_model,
                speaking_rate: 1.0,
                pitch_shift: 0.0,
                volume_gain_db: 0.0,
                sample_rate_hz: 24000,
            });

            SegmentRecord {
                id: Uuid::new_v4().to_string(),
                project_id: proj_id.clone(),
                position: c.position as usize,
                text: c.text.clone(),
                prompt: build_tts_prompt(&c.text, &prompt_opts),
                status: SegmentStatus::Pending,
                attempts: 0,
                audio_path: None,
                duration_ms: c.estimated_duration_ms as u64,
                error_code: None,
                error_message: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                fingerprint: Some(fp),
                output_fingerprint: None,
                attempt_count: 0,
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
                output_size: 0,
                voice: None,
                synthesis_status: Some(SynthesisStatus::Pending),
                review_status: Some(ReviewStatus::Unreviewed),
                reviewed_output_fingerprint: None,
            }
        })
        .collect();

    ProjectRepository::insert_segments(db, &segments).map_err(AppError::DatabaseError)?;
    Ok(proj)
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> AppResult<Vec<ProjectRecord>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::list_projects(db).map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn delete_project(project_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::delete_project(db, &project_id).map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn delete_projects_batch(
    project_ids: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::delete_projects_batch(db, &project_ids).map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn delete_segment(
    project_id: String,
    segment_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::delete_segment(db, &project_id, &segment_id).map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn delete_segments_batch(
    project_id: String,
    segment_ids: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::delete_segments_batch(db, &project_id, &segment_ids)
        .map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn insert_segment_at(
    project_id: String,
    position: usize,
    text: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::insert_segment_at(db, &project_id, position, &text)
        .map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn move_segment(
    project_id: String,
    segment_id: String,
    direction: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::swap_segment_positions(db, &project_id, &segment_id, &direction)
        .map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn get_project_segments(
    project_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<SegmentRecord>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::get_segments_for_project(db, &project_id).map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn normalize_vietnamese_text(text: String) -> AppResult<String> {
    Ok(crate::text::normalizer::VietnameseNormalizer::normalize(
        &text,
    ))
}

#[tauri::command]
pub fn update_segment_text(
    app: tauri::AppHandle,
    project_id: String,
    segment_id: String,
    text: String,
) -> AppResult<()> {
    let state = app.state::<AppState>();
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;

    let proj =
        ProjectRepository::get_project_by_id(db, &project_id).map_err(AppError::DatabaseError)?;
    let (voice, preset, model) = proj
        .map(|p| (p.voice, p.preset, p.model))
        .unwrap_or_else(|| {
            (
                "Kore".to_string(),
                "Tự nhiên".to_string(),
                MODEL_GEMINI_31_FLASH_TTS.to_string(),
            )
        });

    let prompt_opts = PromptStyleOptions {
        style_preset: preset,
        pacing: "Bình thường".to_string(),
        pronunciation_notes: None,
    };
    let prompt = build_tts_prompt(&text, &prompt_opts);
    let fp = compute_segment_fingerprint(&SegmentFingerprintInput {
        text: &text,
        voice: &voice,
        model: &model,
        speaking_rate: 1.0,
        pitch_shift: 0.0,
        volume_gain_db: 0.0,
        sample_rate_hz: 24000,
    });

    SegmentRepository::update_text(db, &project_id, &segment_id, &text, &prompt, &fp)
        .map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn update_project_voice(
    app: tauri::AppHandle,
    project_id: String,
    voice_id: String,
) -> AppResult<()> {
    let state = app.state::<AppState>();
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::update_voice(db, &project_id, &voice_id).map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn split_segment(
    app: tauri::AppHandle,
    project_id: String,
    segment_id: String,
    split_index: usize,
) -> AppResult<()> {
    let state = app.state::<AppState>();
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    SegmentRepository::split_segment(db, &project_id, &segment_id, split_index)
        .map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn merge_segments(
    app: tauri::AppHandle,
    project_id: String,
    segment_id: String,
) -> AppResult<()> {
    let state = app.state::<AppState>();
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    SegmentRepository::merge_with_previous(db, &project_id, &segment_id)
        .map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn chunk_text_preview(
    text: String,
    mode: Option<String>,
) -> AppResult<Vec<crate::text::chunker::TextChunk>> {
    validate_source_text(&text).map_err(AppError::ValidationFailed)?;
    validate_chunk_mode(mode.as_deref()).map_err(AppError::ValidationFailed)?;
    let mode_str = mode.as_deref().unwrap_or("auto");
    Ok(crate::text::chunker::chunk_vietnamese_text_by_mode(
        &text, mode_str,
    ))
}

#[tauri::command]
pub fn rechunk_project_segments(
    app: tauri::AppHandle,
    project_id: String,
    source_text: String,
    mode: Option<String>,
) -> AppResult<Vec<SegmentRecord>> {
    validate_source_text(&source_text).map_err(AppError::ValidationFailed)?;
    validate_chunk_mode(mode.as_deref()).map_err(AppError::ValidationFailed)?;

    let state = app.state::<AppState>();
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    let proj =
        ProjectRepository::get_project_by_id(db, &project_id).map_err(AppError::DatabaseError)?;
    let (voice, preset, model) = proj
        .map(|p| (p.voice, p.preset, p.model))
        .unwrap_or_else(|| {
            (
                "Kore".to_string(),
                "Tự nhiên".to_string(),
                MODEL_GEMINI_31_FLASH_TTS.to_string(),
            )
        });

    let mode_str = mode.as_deref().unwrap_or("auto");
    let chunks = crate::text::chunker::chunk_vietnamese_text_by_mode(&source_text, mode_str);

    ProjectRepository::update_source_text(db, &project_id, &source_text)
        .map_err(AppError::DatabaseError)?;
    ProjectRepository::delete_segments_for_project(db, &project_id)
        .map_err(AppError::DatabaseError)?;

    let now = Utc::now().to_rfc3339();
    let prompt_opts = PromptStyleOptions {
        style_preset: preset,
        pacing: "Bình thường".to_string(),
        pronunciation_notes: None,
    };

    let segments: Vec<SegmentRecord> = chunks
        .into_iter()
        .map(|c| {
            let fp = compute_segment_fingerprint(&SegmentFingerprintInput {
                text: &c.text,
                voice: &voice,
                model: &model,
                speaking_rate: 1.0,
                pitch_shift: 0.0,
                volume_gain_db: 0.0,
                sample_rate_hz: 24000,
            });

            SegmentRecord {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                position: c.position as usize,
                text: c.text.clone(),
                prompt: build_tts_prompt(&c.text, &prompt_opts),
                status: SegmentStatus::Pending,
                attempts: 0,
                audio_path: None,
                duration_ms: c.estimated_duration_ms as u64,
                error_code: None,
                error_message: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                fingerprint: Some(fp),
                output_fingerprint: None,
                attempt_count: 0,
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
                output_size: 0,
                voice: None,
                synthesis_status: Some(SynthesisStatus::Pending),
                review_status: Some(ReviewStatus::Unreviewed),
                reviewed_output_fingerprint: None,
            }
        })
        .collect();

    ProjectRepository::insert_segments(db, &segments).map_err(AppError::DatabaseError)?;
    ProjectRepository::get_segments_for_project(db, &project_id).map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn update_segment_voice(
    project_id: String,
    segment_id: String,
    voice: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::update_segment_voice(db, &project_id, &segment_id, voice.as_deref())
        .map_err(AppError::DatabaseError)
}

#[tauri::command]
pub fn update_segment_review_status(
    segment_id: String,
    review_status: String,
    reviewed_output_fingerprint: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;
    ProjectRepository::update_segment_review_status(
        db,
        &segment_id,
        &review_status,
        reviewed_output_fingerprint.as_deref(),
    )
    .map_err(AppError::DatabaseError)
}
