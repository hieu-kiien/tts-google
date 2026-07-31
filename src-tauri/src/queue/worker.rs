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
use crate::storage::audio_cache::AudioCache;
use crate::api::interactions_client::ApiError;
use crate::audio::pcm_wav::{write_pcm_to_wav_file, get_wav_duration_ms};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum QueueState {
    Idle,
    Running,
    Paused,
    Cancelled,
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

        tauri::async_runtime::spawn(async move {

            let mut current_project_id: Option<String> = None;
            let mut cancel_token: Option<CancellationToken> = None;
            let mut is_paused = false;

            info!("QueueService actor loop started with stream_id: {}", stream_id_clone);

            // Recover expired/orphaned jobs on startup
            if let Some(db) = &app_state.db {
                let _ = db.recover_expired_jobs();
            }

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    QueueCommand::EnqueueProject { project_id, reply } => {
                        // Idempotent check
                        if current_project_id.as_deref() == Some(&project_id) && !is_paused {
                            let snapshot = Self::build_snapshot(&app_state, &project_id);
                            let _ = reply.send(Ok(snapshot));
                            continue;
                        }

                        // Mark segments as queued in DB
                        if let Some(db) = &app_state.db {
                            let _ = ProjectRepository::mark_project_segments_queued(db, &project_id);
                        }

                        current_project_id = Some(project_id.clone());
                        is_paused = false;

                        // Cancel previous token if any
                        if let Some(token) = cancel_token.take() {
                            token.cancel();
                        }
                        let token = CancellationToken::new();
                        cancel_token = Some(token.clone());

                        let snapshot = Self::build_snapshot(&app_state, &project_id);
                        let _ = reply.send(Ok(snapshot));

                        let app_state_runner = Arc::clone(&app_state);
                        let handle_runner = handle_clone.clone();
                        let stream_id_runner = stream_id_clone.clone();
                        let project_id_runner = project_id.clone();

                        tauri::async_runtime::spawn(async move {

                            Self::run_worker_loop(
                                project_id_runner,
                                app_state_runner,
                                handle_runner,
                                stream_id_runner,
                                token,
                            ).await;
                        });
                    }
                    QueueCommand::PauseProject { project_id, reply } => {
                        if current_project_id.as_deref() == Some(&project_id) {
                            is_paused = true;
                            if let Some(token) = cancel_token.take() {
                                token.cancel();
                            }
                        }
                        let _ = reply.send(Ok(()));
                    }
                    QueueCommand::ResumeProject { project_id, reply } => {
                        if current_project_id.as_deref() == Some(&project_id) && is_paused {
                            is_paused = false;
                            let token = CancellationToken::new();
                            cancel_token = Some(token.clone());

                            let app_state_runner = Arc::clone(&app_state);
                            let handle_runner = handle_clone.clone();
                            let stream_id_runner = stream_id_clone.clone();
                            let project_id_runner = project_id.clone();

                            tauri::async_runtime::spawn(async move {

                                Self::run_worker_loop(
                                    project_id_runner,
                                    app_state_runner,
                                    handle_runner,
                                    stream_id_runner,
                                    token,
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
                        }
                        let _ = reply.send(Ok(()));
                    }
                    QueueCommand::GetSnapshot { project_id, reply } => {
                        let snapshot = Self::build_snapshot(&app_state, &project_id);
                        let _ = reply.send(Ok(snapshot));
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

    fn build_snapshot(app_state: &AppState, project_id: &str) -> QueueSnapshot {
        let db = match &app_state.db {
            Some(d) => d,
            None => return QueueSnapshot {
                project_id: project_id.to_string(),
                queue_state: QueueState::Idle,
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

        QueueSnapshot {
            project_id: project_id.to_string(),
            queue_state: if pending > 0 { QueueState::Running } else { QueueState::Idle },
            total_segments: total,
            completed_segments: completed,
            failed_segments: failed,
            pending_segments: pending,
            snapshot_revision: max_rev,
        }
    }

    /// Computes current segment counts from DB for accurate progress events.
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
    ) {
        let worker_id = format!("worker_{}", Uuid::new_v4().simple());
        let lease_duration_secs = 45u64;
        let mut sequence: u64 = 0;

        loop {
            if cancel_token.is_cancelled() {
                info!("Worker loop cancelled for project {}", project_id);
                break;
            }

            let db = match &app_state.db {
                Some(d) => d,
                None => break,
            };

            // 1. Claim next task from DB
            let task = match ProjectRepository::claim_next_task(db, &project_id, &worker_id, lease_duration_secs) {
                Ok(Some(t)) => t,
                Ok(None) => {
                    if let Ok(Some(delay_ms)) = ProjectRepository::get_next_retry_delay_ms(db, &project_id) {
                        let wait_time = delay_ms.clamp(100, 2000);
                        info!("No immediate task ready for project {}, but retry_wait tasks exist. Sleeping {}ms...", project_id, wait_time);
                        sleep(Duration::from_millis(wait_time)).await;
                        continue;
                    }
                    info!("No more queued or retrying tasks for project {}. Worker exiting loop.", project_id);
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

            // 2. LOCK RELEASE PATTERN: Perform TTS Network Request outside database locks
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
                break;
            }

            let (target_voice, target_model) = if let Some(db) = &app_state.db {
                if let Ok(Some(p)) = ProjectRepository::get_project_by_id(db, &project_id) {
                    (p.voice, p.model)
                } else {
                    ("Kore".to_string(), "gemini-3.1-flash-tts-preview".to_string())
                }
            } else {
                ("Kore".to_string(), "gemini-3.1-flash-tts-preview".to_string())
            };

            // --- CACHE CHECK: skip API if identical audio exists ---
            let cache_key = AudioCache::compute_cache_key(&target_model, &target_voice, &task.text);
            if let Some(db) = &app_state.db {
                if let Some(cached_path) = AudioCache::lookup(db, &cache_key) {
                    let target_path = format!("{}/seg_{}_{}.wav", app_state.output_dir, project_id, task.position);
                    if cached_path != target_path {
                        let _ = std::fs::copy(&cached_path, &target_path);
                    }
                    let duration_ms = get_wav_duration_ms(&target_path).unwrap_or(0);
                    let byte_size = std::fs::metadata(&target_path).map(|m| m.len()).unwrap_or(0);

                    let committed = ProjectRepository::commit_task_result(
                        db, &task.id, &worker_id, task.fingerprint.as_deref(),
                        "success", Some(&target_path), byte_size, duration_ms,
                        None, None, None,
                    ).unwrap_or(false);

                    if committed {
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
                    // Skip API call and rate-limit sleep
                    continue;
                }
            }

            let tts_res = tokio::select! {
                result = app_state.gemini_client.synthesize_speech(
                    &api_key,
                    &target_model,
                    &task.prompt,
                    &target_voice,
                ) => result,
                _ = cancel_token.cancelled() => {
                    info!("API request cancelled for task {} during synthesis.", task.id);
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
                    break;
                }
            };

            // 3. Process API result and commit conditionally to DB
            match tts_res {
                Ok(pcm_bytes) => {
                    let target_path = format!("{}/seg_{}_{}.wav", app_state.output_dir, project_id, task.position);
                    let target_path_tmp = format!("{}.tmp", target_path);

                    if write_pcm_to_wav_file(&pcm_bytes, &target_path_tmp).is_ok() {
                        // Atomic rename in same directory
                        if let Err(rename_err) = std::fs::rename(&target_path_tmp, &target_path) {
                            tracing::info!("Worker rename failed ({}), falling back to copy+remove.", rename_err);
                            if let Err(copy_err) = std::fs::copy(&target_path_tmp, &target_path) {
                                tracing::warn!("Worker copy also failed: {}", copy_err);
                            }
                            let _ = std::fs::remove_file(&target_path_tmp);
                        }
                        let duration_ms = get_wav_duration_ms(&target_path).unwrap_or(0);
                        let bytes_len = pcm_bytes.len() as u64;

                        // Store in audio cache for future reuse
                        if let Some(db) = &app_state.db {
                            let _ = AudioCache::store(
                                db, &cache_key, &target_model, &target_voice,
                                &target_path, duration_ms, bytes_len,
                            );
                        }

                        let committed = ProjectRepository::commit_task_result(
                            db,
                            &task.id,
                            &worker_id,
                            task.fingerprint.as_deref(),
                            "success",
                            Some(&target_path),
                            bytes_len,
                            duration_ms,
                            None,
                            None,
                            None,
                        ).unwrap_or(false);

                        if !committed {
                            warn!("Commit result returned false for task {}. Output discarded (stale/reclaimed).", task.id);
                            let _ = std::fs::remove_file(&target_path);
                        } else {
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

                    let is_retryable = match &err {
                        ApiError::RateLimited(retry_after) => {
                            // Mark this key on cooldown so next request uses a different key
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
                            // Mark this key exhausted for 24h, try next key
                            app_state.credentials.mark_key_daily_exhausted(&api_key);
                            if app_state.credentials.key_count() > 1 {
                                warn!("Key daily limit hit — rotating to next key for project {}.", project_id);
                                true // retry with next key
                            } else {
                                warn!("Daily quota exhausted (single key) — pausing queue for project {}.", project_id);
                                false // no other key available
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

            // Dynamic rate limit delay: reduce per-request pause when multiple API keys are available
            let key_count = app_state.credentials.key_count().max(1) as u64;
            let rate_delay_ms = (4100u64 / key_count).max(500); // minimum 500ms between requests
            tokio::select! {
                _ = sleep(Duration::from_millis(rate_delay_ms)) => {},
                _ = cancel_token.cancelled() => {
                    info!("Rate-limit sleep interrupted by cancellation.");
                    break;
                }
            }
        }
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
        assert_eq!(calculate_backoff_delay(&net_err, 0), 2); // 2^1 = 2
        assert_eq!(calculate_backoff_delay(&net_err, 1), 4); // 2^2 = 4
        assert_eq!(calculate_backoff_delay(&net_err, 2), 8); // 2^3 = 8
    }
}

