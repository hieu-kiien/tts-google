use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use tauri::Emitter;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::Utc;

use crate::state::app_state::AppState;
use crate::storage::project_repo::ProjectRepository;
use crate::api::interactions_client::ApiError;
use crate::audio::pcm_wav::{write_pcm_to_wav_file, get_wav_duration_ms};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum QueueState {
    Idle,
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerOutcome {
    Completed,
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueProgressEvent {
    pub stream_id: String,
    pub sequence: u64,
    pub project_id: String,
    pub segment_id: Option<String>,
    pub position: usize,
    pub total_segments: usize,
    pub completed_segments: usize,
    pub status: String,
    pub revision: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueSnapshot {
    pub project_id: String,
    pub queue_state: QueueState,
    pub total_segments: usize,
    pub completed_segments: usize,
    pub failed_segments: usize,
    pub pending_segments: usize,
    pub snapshot_revision: u64,
}

pub enum QueueCommand {
    EnqueueProject { project_id: String, reply: oneshot::Sender<Result<QueueSnapshot, String>> },
    PauseProject { project_id: String, reply: oneshot::Sender<Result<(), String>> },
    ResumeProject { project_id: String, reply: oneshot::Sender<Result<(), String>> },
    CancelProject { project_id: String, reply: oneshot::Sender<Result<(), String>> },
    GetSnapshot { project_id: String, reply: oneshot::Sender<Result<QueueSnapshot, String>> },
    WorkerFinished { project_id: String, outcome: WorkerOutcome },
}

pub struct QueueService {
    tx: mpsc::Sender<QueueCommand>,
    #[allow(dead_code)]
    stream_id: String,
}

impl QueueService {
    pub fn new(app_state: Arc<AppState>, handle: tauri::AppHandle) -> Self {
        let stream_id = Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel::<QueueCommand>(100);

        let stream_id_clone = stream_id.clone();
        let handle_clone = handle.clone();
        let tx_for_worker = tx.clone();

        tauri::async_runtime::spawn(async move {
            let mut current_project_id: Option<String> = None;
            let mut cancel_token: Option<CancellationToken> = None;
            let mut is_paused = false;

            info!("QueueService actor loop started with stream_id: {}", stream_id_clone);

            if let Some(db) = &app_state.db {
                let _ = db.recover_expired_jobs();
            }

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    QueueCommand::EnqueueProject { project_id, reply } => {
                        // Idempotent check
                        if current_project_id.as_deref() == Some(&project_id) && !is_paused {
                            let snapshot = Self::build_snapshot(&app_state, &project_id, Some(QueueState::Running));
                            let _ = reply.send(Ok(snapshot));
                            continue;
                        }

                        if let Some(db) = &app_state.db {
                            let _ = ProjectRepository::mark_project_segments_queued(db, &project_id);
                        }

                        current_project_id = Some(project_id.clone());
                        is_paused = false;

                        if let Some(token) = cancel_token.take() {
                            token.cancel();
                        }
                        let token = CancellationToken::new();
                        cancel_token = Some(token.clone());

                        let snapshot = Self::build_snapshot(&app_state, &project_id, Some(QueueState::Running));
                        let _ = reply.send(Ok(snapshot.clone()));
                        let _ = handle_clone.emit("queue-snapshot", &snapshot);

                        let app_state_runner = Arc::clone(&app_state);
                        let handle_runner = handle_clone.clone();
                        let stream_id_runner = stream_id_clone.clone();
                        let project_id_runner = project_id.clone();
                        let tx_runner = tx_for_worker.clone();

                        tauri::async_runtime::spawn(async move {
                            Self::run_worker_loop(
                                project_id_runner,
                                app_state_runner,
                                handle_runner,
                                stream_id_runner,
                                token,
                                tx_runner,
                            ).await;
                        });
                    }
                    QueueCommand::PauseProject { project_id, reply } => {
                        if current_project_id.as_deref() == Some(&project_id) {
                            is_paused = true;
                            if let Some(token) = cancel_token.take() {
                                token.cancel();
                            }
                            let snapshot = Self::build_snapshot(&app_state, &project_id, Some(QueueState::Paused));
                            let _ = handle_clone.emit("queue-snapshot", &snapshot);
                        }
                        let _ = reply.send(Ok(()));
                    }
                    QueueCommand::ResumeProject { project_id, reply } => {
                        if current_project_id.as_deref() == Some(&project_id) && is_paused {
                            is_paused = false;
                            let token = CancellationToken::new();
                            cancel_token = Some(token.clone());

                            let snapshot = Self::build_snapshot(&app_state, &project_id, Some(QueueState::Running));
                            let _ = handle_clone.emit("queue-snapshot", &snapshot);

                            let app_state_runner = Arc::clone(&app_state);
                            let handle_runner = handle_clone.clone();
                            let stream_id_runner = stream_id_clone.clone();
                            let project_id_runner = project_id.clone();
                            let tx_runner = tx_for_worker.clone();

                            tauri::async_runtime::spawn(async move {
                                Self::run_worker_loop(
                                    project_id_runner,
                                    app_state_runner,
                                    handle_runner,
                                    stream_id_runner,
                                    token,
                                    tx_runner,
                                ).await;
                            });
                        }
                        let _ = reply.send(Ok(()));
                    }
                    QueueCommand::CancelProject { project_id, reply } => {
                        if current_project_id.as_deref() == Some(&project_id) {
                            if let Some(token) = cancel_token.take() {
                                token.cancel();
                            }
                            current_project_id = None;
                            is_paused = false;

                            let snapshot = Self::build_snapshot(&app_state, &project_id, Some(QueueState::Cancelled));
                            let _ = handle_clone.emit("queue-snapshot", &snapshot);
                        }
                        let _ = reply.send(Ok(()));
                    }
                    QueueCommand::GetSnapshot { project_id, reply } => {
                        let override_st = if current_project_id.as_deref() == Some(&project_id) {
                            if is_paused {
                                Some(QueueState::Paused)
                            } else {
                                Some(QueueState::Running)
                            }
                        } else {
                            None
                        };
                        let snapshot = Self::build_snapshot(&app_state, &project_id, override_st);
                        let _ = reply.send(Ok(snapshot));
                    }
                    QueueCommand::WorkerFinished { project_id, outcome } => {
                        if current_project_id.as_deref() == Some(&project_id) && !is_paused {
                            current_project_id = None;
                            cancel_token = None;
                            let final_state = match outcome {
                                WorkerOutcome::Completed => QueueState::Completed,
                                WorkerOutcome::Cancelled => QueueState::Cancelled,
                                WorkerOutcome::Failed(_) => QueueState::Failed,
                            };

                            let snapshot = Self::build_snapshot(&app_state, &project_id, Some(final_state));
                            let _ = handle_clone.emit("queue-snapshot", &snapshot);
                        }
                    }
                }
            }
        });

        Self { tx, stream_id }
    }

    pub async fn enqueue_project(&self, project_id: String) -> Result<QueueSnapshot, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(QueueCommand::EnqueueProject { project_id, reply: reply_tx })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx.await.map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn pause_project(&self, project_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(QueueCommand::PauseProject { project_id, reply: reply_tx })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx.await.map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn resume_project(&self, project_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(QueueCommand::ResumeProject { project_id, reply: reply_tx })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx.await.map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn cancel_project(&self, project_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(QueueCommand::CancelProject { project_id, reply: reply_tx })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx.await.map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn get_queue_snapshot(&self, project_id: String) -> Result<QueueSnapshot, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(QueueCommand::GetSnapshot { project_id, reply: reply_tx })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx.await.map_err(|_| "Actor dropped reply".to_string())?
    }

    fn build_snapshot(app_state: &AppState, project_id: &str, override_state: Option<QueueState>) -> QueueSnapshot {
        let db = match &app_state.db {
            Some(d) => d,
            None => return QueueSnapshot {
                project_id: project_id.to_string(),
                queue_state: override_state.unwrap_or(QueueState::Idle),
                total_segments: 0,
                completed_segments: 0,
                failed_segments: 0,
                pending_segments: 0,
                snapshot_revision: 0,
            },
        };

        let segs = ProjectRepository::get_segments_for_project(db, project_id).unwrap_or_default();
        let total = segs.len();
        let completed = segs.iter().filter(|s| s.status == "success").count();
        let failed = segs.iter().filter(|s| s.status == "failed").count();
        let pending = total.saturating_sub(completed + failed);
        let max_rev = segs.iter().map(|s| s.state_revision).max().unwrap_or(0);

        let queue_state = if let Some(st) = override_state {
            st
        } else if pending == 0 && total > 0 && failed == 0 {
            QueueState::Completed
        } else if failed > 0 && pending == 0 {
            QueueState::Failed
        } else {
            QueueState::Idle
        };

        QueueSnapshot {
            project_id: project_id.to_string(),
            queue_state,
            total_segments: total,
            completed_segments: completed,
            failed_segments: failed,
            pending_segments: pending,
            snapshot_revision: max_rev,
        }
    }

    fn get_segment_counts(app_state: &AppState, project_id: &str) -> (usize, usize, usize) {
        let db = match &app_state.db {
            Some(d) => d,
            None => return (0, 0, 0),
        };
        let segs = ProjectRepository::get_segments_for_project(db, project_id).unwrap_or_default();
        let total = segs.len();
        let completed = segs.iter().filter(|s| s.status == "success").count();
        let failed = segs.iter().filter(|s| s.status == "failed").count();
        (total, completed, failed)
    }

    async fn run_worker_loop(
        project_id: String,
        app_state: Arc<AppState>,
        handle: tauri::AppHandle,
        stream_id: String,
        cancel_token: CancellationToken,
        tx: mpsc::Sender<QueueCommand>,
    ) {
        let worker_id = format!("worker_{}", Uuid::new_v4().simple());
        let lease_duration_secs = 45u64;
        let mut sequence: u64 = 0;
        let final_outcome;

        loop {
            if cancel_token.is_cancelled() {
                info!("Worker loop cancelled for project {}", project_id);
                final_outcome = WorkerOutcome::Cancelled;
                break;
            }

            let db = match &app_state.db {
                Some(d) => d,
                None => {
                    final_outcome = WorkerOutcome::Failed("Database offline".to_string());
                    break;
                }
            };

            let task = match ProjectRepository::claim_next_task(db, &project_id, &worker_id, lease_duration_secs) {
                Ok(Some(t)) => t,
                Ok(None) => {
                    if let Ok(Some(delay_ms)) = ProjectRepository::get_next_retry_delay_ms(db, &project_id) {
                        let wait_time = delay_ms.clamp(100, 2000);
                        info!("No immediate task ready for project {}, sleeping {}ms...", project_id, wait_time);
                        sleep(Duration::from_millis(wait_time)).await;
                        continue;
                    }
                    info!("No more queued tasks for project {}. Worker exiting loop.", project_id);
                    let (total, comp, fail) = Self::get_segment_counts(&app_state, &project_id);
                    if fail > 0 && comp + fail >= total {
                        final_outcome = WorkerOutcome::Failed(format!("Queue finished with {} failed segments", fail));
                    } else {
                        final_outcome = WorkerOutcome::Completed;
                    }
                    break;
                }
                Err(e) => {
                    error!("Error claiming task for project {}: {}", project_id, e);
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            sequence += 1;
            let (total_segs, completed_segs, _) = Self::get_segment_counts(&app_state, &project_id);
            let _ = handle.emit("queue-progress", QueueProgressEvent {
                stream_id: stream_id.clone(),
                sequence,
                project_id: project_id.clone(),
                segment_id: Some(task.id.clone()),
                position: task.position,
                total_segments: total_segs,
                completed_segments: completed_segs,
                status: "processing".to_string(),
                revision: task.state_revision,
                error_message: None,
            });

            let api_key = match app_state.credentials.get_key() {
                Some(k) => k,
                None => {
                    let _ = ProjectRepository::commit_task_result(
                        db,
                        &task.id,
                        &worker_id,
                        None,
                        "failed",
                        None,
                        0,
                        0,
                        None,
                        Some("401"),
                        Some("Missing Gemini API Key"),
                    );
                    final_outcome = WorkerOutcome::Failed("Missing Gemini API Key".to_string());
                    break;
                }
            };

            if cancel_token.is_cancelled() {
                let _ = ProjectRepository::commit_task_result(
                    db,
                    &task.id,
                    &worker_id,
                    None,
                    "queued",
                    None,
                    0,
                    0,
                    None,
                    None,
                    None,
                );
                final_outcome = WorkerOutcome::Cancelled;
                break;
            }

            let proj_record = ProjectRepository::get_project_by_id(db, &project_id).ok().flatten();
            let model_name = proj_record.map(|p| p.model).unwrap_or_else(|| "gemini-3.1-flash-tts-preview".to_string());

            let start_inst = std::time::Instant::now();
            let pcm_result = app_state
                .gemini_client
                .synthesize_speech(&api_key, &model_name, &task.prompt, &task.voice.clone().unwrap_or_else(|| "Kore".to_string()))
                .await;
            let latency_ms = start_inst.elapsed().as_millis() as u64;
            let char_count = task.text.chars().count();

            match pcm_result {
                Ok(pcm_bytes) => {
                    app_state.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    app_state.total_chars.fetch_add(char_count as u64, std::sync::atomic::Ordering::Relaxed);
                    app_state.total_latency_ms.fetch_add(latency_ms, std::sync::atomic::Ordering::Relaxed);
                    if let Some(ref db_instance) = app_state.db {
                        let _ = db_instance.record_quota_metric(char_count, false, latency_ms);
                    }

                    let fp = task.fingerprint.as_deref().unwrap_or("legacy_fp");
                    let cached_wav_path = app_state.output_dir.join(format!("{}.wav", fp));
                    let wav_path_str = cached_wav_path.to_string_lossy().to_string();

                    if let Err(e) = write_pcm_to_wav_file(&pcm_bytes, &wav_path_str) {
                        error!("Failed to write WAV file: {}", e);
                    }

                    if cached_wav_path.exists() {
                        let dur_ms = get_wav_duration_ms(&wav_path_str).unwrap_or(task.duration_ms);
                        let file_size = std::fs::metadata(&cached_wav_path).map(|m| m.len()).unwrap_or(0);

                        let commit_res = ProjectRepository::commit_task_result(
                            db,
                            &task.id,
                            &worker_id,
                            Some(&wav_path_str),
                            "success",
                            Some(fp),
                            dur_ms,
                            file_size,
                            None,
                            None,
                            None,
                        );

                        if commit_res.is_ok() {
                            sequence += 1;
                            let (t_segs, c_segs, _) = Self::get_segment_counts(&app_state, &project_id);
                            let _ = handle.emit("queue-progress", QueueProgressEvent {
                                stream_id: stream_id.clone(),
                                sequence,
                                project_id: project_id.clone(),
                                segment_id: Some(task.id.clone()),
                                position: task.position,
                                total_segments: t_segs,
                                completed_segments: c_segs,
                                status: "success".to_string(),
                                revision: task.state_revision + 1,
                                error_message: None,
                            });
                        }
                    } else {
                        let _ = ProjectRepository::commit_task_result(
                            db,
                            &task.id,
                            &worker_id,
                            None,
                            "failed",
                            None,
                            0,
                            0,
                            None,
                            Some("FS_ERROR"),
                            Some("Failed to write temporary WAV file"),
                        );
                    }
                }
                Err(err) => {
                    let err_str = err.to_string();
                    warn!("Task {} attempt {} failed: {}", task.id, task.attempt_count + 1, err_str);

                    let is_rate_limit = matches!(err, ApiError::RateLimited(_) | ApiError::RateLimitedDaily);
                    if is_rate_limit {
                        app_state.rate_limit_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if let Some(ref db_instance) = app_state.db {
                            let _ = db_instance.record_quota_metric(0, true, latency_ms);
                        }
                    }

                    let is_retryable = match &err {
                        ApiError::RateLimited(retry_after) => {
                            let cooldown = retry_after.unwrap_or(30);
                            app_state.credentials.mark_key_cooldown(&api_key, cooldown);
                            true
                        },
                        ApiError::NetworkError(_) => true,
                        ApiError::ApiServerError(code, _) => *code == 408 || *code >= 500,
                        ApiError::MissingAudio => true,
                        ApiError::CorruptAudio(_) => true,
                        ApiError::TruncatedAudio { .. } => true,
                        ApiError::RateLimitedDaily => {
                            app_state.credentials.mark_key_daily_exhausted(&api_key);
                            if app_state.credentials.key_count() > 1 {
                                warn!("Key daily limit hit — rotating key for project {}.", project_id);
                                true
                            } else {
                                warn!("Daily quota exhausted — pausing queue for project {}.", project_id);
                                false
                            }
                        },
                        _ => false,
                    };

                    if is_retryable && task.attempt_count < 3 {
                        let base_delay = match &err {
                            ApiError::RateLimited(Some(secs)) => *secs,
                            ApiError::RateLimited(None) => 10,
                            _ => u64::pow(2, task.attempt_count + 1),
                        };
                        let jitter_ms = (Utc::now().timestamp_subsec_millis() as u64 % 1000) + 500;
                        let now_ms = Utc::now().timestamp_millis();
                        let next_retry = now_ms + (base_delay * 1000 + jitter_ms) as i64;
                        let err_code = match &err {
                            ApiError::RateLimited(_) => "RESOURCE_EXHAUSTED",
                            _ => "RETRYABLE_ERROR",
                        };

                        let _ = ProjectRepository::commit_task_result(
                            db,
                            &task.id,
                            &worker_id,
                            None,
                            "retry_wait",
                            None,
                            0,
                            0,
                            Some(next_retry),
                            Some(err_code),
                            Some(&err_str),
                        );
                    } else {
                        let _ = ProjectRepository::commit_task_result(
                            db,
                            &task.id,
                            &worker_id,
                            None,
                            "failed",
                            None,
                            0,
                            0,
                            None,
                            Some("TERMINAL_ERROR"),
                            Some(&err_str),
                        );
                    }
                }
            }

            let key_count = app_state.credentials.key_count().max(1) as u64;
            let rate_delay_ms = (4100u64 / key_count).max(500);
            tokio::select! {
                _ = sleep(Duration::from_millis(rate_delay_ms)) => {},
                _ = cancel_token.cancelled() => {
                    info!("Rate-limit sleep interrupted by cancellation.");
                    final_outcome = WorkerOutcome::Cancelled;
                    break;
                }
            }
        }

        // Send WorkerFinished to actor before exiting
        let _ = tx.send(QueueCommand::WorkerFinished { project_id, outcome: final_outcome }).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::api::interactions_client::ApiError;

    fn is_error_retryable(err: &ApiError) -> bool {
        match err {
            ApiError::RateLimited(_) => true,
            ApiError::NetworkError(_) => true,
            ApiError::ApiServerError(code, _) => *code == 408 || *code >= 500,
            _ => false,
        }
    }

    fn calculate_backoff_delay(err: &ApiError, attempt: u32) -> u64 {
        match err {
            ApiError::RateLimited(Some(secs)) => *secs,
            ApiError::RateLimited(None) => 10,
            _ => u64::pow(2, attempt + 1),
        }
    }

    #[test]
    fn test_retry_classification() {
        assert!(is_error_retryable(&ApiError::RateLimited(Some(30))));
        assert!(is_error_retryable(&ApiError::NetworkError("Timeout".to_string())));
        assert!(is_error_retryable(&ApiError::ApiServerError(500, "Internal Server Error".to_string())));
        assert!(is_error_retryable(&ApiError::ApiServerError(503, "Service Unavailable".to_string())));

        assert!(!is_error_retryable(&ApiError::Unauthorized));
        assert!(!is_error_retryable(&ApiError::MissingAudio));
        assert!(!is_error_retryable(&ApiError::ApiServerError(400, "Bad Request".to_string())));
        assert!(!is_error_retryable(&ApiError::ApiServerError(403, "Forbidden".to_string())));
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let rate_limit_header = ApiError::RateLimited(Some(45));
        assert_eq!(calculate_backoff_delay(&rate_limit_header, 0), 45);

        let rate_limit_no_header = ApiError::RateLimited(None);
        assert_eq!(calculate_backoff_delay(&rate_limit_no_header, 0), 10);

        let net_err = ApiError::NetworkError("Connect failed".to_string());
        assert_eq!(calculate_backoff_delay(&net_err, 0), 2);
        assert_eq!(calculate_backoff_delay(&net_err, 1), 4);
        assert_eq!(calculate_backoff_delay(&net_err, 2), 8);
    }
}
