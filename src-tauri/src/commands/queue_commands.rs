use crate::queue::worker::QueueSnapshot;
use crate::state::app_state::AppState;
use crate::storage::project_repo::ProjectRepository;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub diagnostic_id: Option<String>,
}

impl From<String> for CommandError {
    fn from(msg: String) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: msg,
            retryable: false,
            diagnostic_id: None,
        }
    }
}

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
) -> Result<QueueSnapshot, CommandError> {
    let queue = state.queue_service.as_ref().ok_or_else(|| CommandError {
        code: "SERVICE_UNAVAILABLE".to_string(),
        message: "Queue service not initialized".to_string(),
        retryable: false,
        diagnostic_id: None,
    })?;

    queue
        .enqueue_project(project_id)
        .await
        .map_err(|e| CommandError {
            code: "ENQUEUE_FAILED".to_string(),
            message: e,
            retryable: true,
            diagnostic_id: None,
        })
}

#[tauri::command]
pub async fn pause_project(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let queue = state.queue_service.as_ref().ok_or_else(|| CommandError {
        code: "SERVICE_UNAVAILABLE".to_string(),
        message: "Queue service not initialized".to_string(),
        retryable: false,
        diagnostic_id: None,
    })?;

    queue.pause_project(project_id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn resume_project(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let queue = state.queue_service.as_ref().ok_or_else(|| CommandError {
        code: "SERVICE_UNAVAILABLE".to_string(),
        message: "Queue service not initialized".to_string(),
        retryable: false,
        diagnostic_id: None,
    })?;

    queue.resume_project(project_id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn cancel_project(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let queue = state.queue_service.as_ref().ok_or_else(|| CommandError {
        code: "SERVICE_UNAVAILABLE".to_string(),
        message: "Queue service not initialized".to_string(),
        retryable: false,
        diagnostic_id: None,
    })?;

    queue.cancel_project(project_id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn get_queue_snapshot(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<QueueSnapshot, CommandError> {
    let queue = state.queue_service.as_ref().ok_or_else(|| CommandError {
        code: "SERVICE_UNAVAILABLE".to_string(),
        message: "Queue service not initialized".to_string(),
        retryable: false,
        diagnostic_id: None,
    })?;

    queue
        .get_queue_snapshot(project_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn check_export_readiness(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<ExportReadiness, CommandError> {
    let db = state.db.as_ref().ok_or_else(|| CommandError {
        code: "DB_UNAVAILABLE".to_string(),
        message: "Database not initialized".to_string(),
        retryable: false,
        diagnostic_id: None,
    })?;

    let segs = ProjectRepository::get_segments_for_project(db, &project_id)
        .map_err(Into::<CommandError>::into)?;
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

    let successful = segs.iter().filter(|s| s.status == "success").count();
    let failed = segs.iter().filter(|s| s.status == "failed").count();
    let processing_or_queued = segs
        .iter()
        .filter(|s| s.status == "processing" || s.status == "queued" || s.status == "retry_wait")
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
) -> Result<(), CommandError> {
    let db = state.db.as_ref().ok_or_else(|| CommandError {
        code: "DB_UNAVAILABLE".to_string(),
        message: "Database not initialized".to_string(),
        retryable: false,
        diagnostic_id: None,
    })?;

    ProjectRepository::requeue_segment(db, &project_id, &segment_id)
        .map_err(|e| CommandError {
            code: "REQUEUE_FAILED".to_string(),
            message: e,
            retryable: false,
            diagnostic_id: None,
        })?;
        
    // Optionally trigger queue if it's paused or we just want to wake it up
    if let Some(queue) = state.queue_service.as_ref() {
        let _ = queue.resume_project(project_id).await;
    }

    Ok(())
}

