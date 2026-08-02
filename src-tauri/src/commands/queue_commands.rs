use crate::error::{AppError, AppResult};
use crate::models::segment::SegmentStatus;
use crate::queue::worker::QueueSnapshot;
use crate::state::app_state::AppState;
use crate::storage::project_repo::ProjectRepository;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportReadiness {
    pub state: String, // "ready" | "queue_not_started" | "queue_running" | "partial" | "empty_project"
    pub successful_segments: usize,
    pub failed_segments: usize,
    pub total_segments: usize,
    pub output_directory: String,
}

#[tauri::command]
pub async fn enqueue_project(
    project_id: String,
    state: State<'_, AppState>,
) -> AppResult<QueueSnapshot> {
    let queue = state
        .queue_service
        .as_ref()
        .ok_or_else(|| AppError::InternalError("Queue service not initialized".to_string()))?;

    queue
        .enqueue_project(project_id)
        .await
        .map_err(AppError::Queue)
}

#[tauri::command]
pub async fn pause_project(project_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let queue = state
        .queue_service
        .as_ref()
        .ok_or_else(|| AppError::InternalError("Queue service not initialized".to_string()))?;

    queue
        .pause_project(project_id)
        .await
        .map_err(AppError::Queue)
}

#[tauri::command]
pub async fn resume_project(project_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let queue = state
        .queue_service
        .as_ref()
        .ok_or_else(|| AppError::InternalError("Queue service not initialized".to_string()))?;

    queue
        .resume_project(project_id)
        .await
        .map_err(AppError::Queue)
}

#[tauri::command]
pub async fn cancel_project(project_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let queue = state
        .queue_service
        .as_ref()
        .ok_or_else(|| AppError::InternalError("Queue service not initialized".to_string()))?;

    queue
        .cancel_project(project_id)
        .await
        .map_err(AppError::Queue)
}

#[tauri::command]
pub async fn get_queue_snapshot(
    project_id: String,
    state: State<'_, AppState>,
) -> AppResult<QueueSnapshot> {
    let queue = state
        .queue_service
        .as_ref()
        .ok_or_else(|| AppError::InternalError("Queue service not initialized".to_string()))?;

    queue
        .get_queue_snapshot(project_id)
        .await
        .map_err(AppError::Queue)
}

#[tauri::command]
pub async fn check_export_readiness(
    project_id: String,
    state: State<'_, AppState>,
) -> AppResult<ExportReadiness> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;

    let segs = ProjectRepository::get_segments_for_project(db, &project_id)
        .map_err(AppError::DatabaseError)?;
    let total = segs.len();
    if total == 0 {
        return Ok(ExportReadiness {
            state: "empty_project".to_string(),
            successful_segments: 0,
            failed_segments: 0,
            total_segments: 0,
            output_directory: state.output_dir.to_string_lossy().to_string(),
        });
    }

    let successful = segs
        .iter()
        .filter(|s| s.status == SegmentStatus::Success || s.status == SegmentStatus::Approved)
        .count();
    let failed = segs
        .iter()
        .filter(|s| s.status == SegmentStatus::Failed)
        .count();
    let processing_or_queued = segs
        .iter()
        .filter(|s| {
            s.status == SegmentStatus::Processing
                || s.status == SegmentStatus::Queued
                || s.status == SegmentStatus::RetryWait
        })
        .count();

    let state_str = if processing_or_queued > 0 {
        "queue_running"
    } else if successful == total {
        "ready"
    } else if successful > 0 {
        "partial"
    } else {
        "queue_not_started"
    };

    Ok(ExportReadiness {
        state: state_str.to_string(),
        successful_segments: successful,
        failed_segments: failed,
        total_segments: total,
        output_directory: state.output_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn requeue_segment(
    project_id: String,
    segment_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::DatabaseError("Database not initialized".to_string()))?;

    ProjectRepository::requeue_segment(db, &project_id, &segment_id)
        .map_err(AppError::DatabaseError)?;

    // Optionally trigger queue if it's paused or we just want to wake it up
    if let Some(queue) = state.queue_service.as_ref() {
        let _ = queue.resume_project(project_id).await;
    }

    Ok(())
}
