use chrono::Utc;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::api::provider::TtsProvider;
use crate::audio::pcm_wav::write_pcm_to_wav_atomic;
use crate::models::segment::SegmentStatus;
use crate::state::app_state::AppState;
use crate::storage::project_repo::ProjectRepository;

const ERROR_RETRY_DELAY_MS: u64 = 500;
const MIN_RATE_DELAY_MS: u64 = 500;

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
    EnqueueProject {
        project_id: String,
        reply: oneshot::Sender<Result<QueueSnapshot, String>>,
    },
    PauseProject {
        project_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    ResumeProject {
        project_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    CancelProject {
        project_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    GetSnapshot {
        project_id: String,
        reply: oneshot::Sender<Result<QueueSnapshot, String>>,
    },
    WorkerFinished {
        project_id: String,
        outcome: WorkerOutcome,
    },
}

pub struct QueueService {
    tx: mpsc::Sender<QueueCommand>,
}

impl QueueService {
    pub fn new(app_state: Arc<AppState>, handle: tauri::AppHandle) -> Self {
        let (tx, mut rx) = mpsc::channel::<QueueCommand>(100);

        let handle_clone = handle.clone();
        let tx_for_worker = tx.clone();

        tauri::async_runtime::spawn(async move {
            let mut current_project_id: Option<String> = None;
            let mut cancel_token: Option<CancellationToken> = None;
            let mut is_paused = false;

            info!("QueueService actor loop started");

            if let Some(db) = &app_state.db {
                let _ = db.recover_expired_jobs();
            }

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    QueueCommand::EnqueueProject { project_id, reply } => {
                        // Idempotent check
                        if current_project_id.as_deref() == Some(&project_id) && !is_paused {
                            let snapshot = Self::build_snapshot(
                                &app_state,
                                &project_id,
                                Some(QueueState::Running),
                            );
                            let _ = reply.send(Ok(snapshot));
                            continue;
                        }

                        if let Some(db) = &app_state.db {
                            let _ =
                                ProjectRepository::mark_project_segments_queued(db, &project_id);
                        }

                        current_project_id = Some(project_id.clone());
                        is_paused = false;

                        if let Some(token) = cancel_token.take() {
                            token.cancel();
                        }
                        let token = CancellationToken::new();
                        cancel_token = Some(token.clone());

                        let snapshot = Self::build_snapshot(
                            &app_state,
                            &project_id,
                            Some(QueueState::Running),
                        );
                        let _ = reply.send(Ok(snapshot.clone()));
                        let _ = handle_clone.emit("queue-snapshot", &snapshot);

                        let worker_count = 2;
                        for _ in 0..worker_count {
                            let app_state_runner = Arc::clone(&app_state);
                            let handle_runner = handle_clone.clone();
                            let project_id_runner = project_id.clone();
                            let tx_runner = tx_for_worker.clone();
                            let token_runner = token.clone();

                            tauri::async_runtime::spawn(async move {
                                Self::run_worker_loop(
                                    project_id_runner,
                                    app_state_runner,
                                    handle_runner,
                                    token_runner,
                                    tx_runner,
                                )
                                .await;
                            });
                        }
                    }
                    QueueCommand::PauseProject { project_id, reply } => {
                        if current_project_id.as_deref() == Some(&project_id) {
                            is_paused = true;
                            if let Some(token) = cancel_token.take() {
                                token.cancel();
                            }
                            let snapshot = Self::build_snapshot(
                                &app_state,
                                &project_id,
                                Some(QueueState::Paused),
                            );
                            let _ = handle_clone.emit("queue-snapshot", &snapshot);
                        }
                        let _ = reply.send(Ok(()));
                    }
                    QueueCommand::ResumeProject { project_id, reply } => {
                        if current_project_id.as_deref() == Some(&project_id) && is_paused {
                            is_paused = false;
                            let token = CancellationToken::new();
                            cancel_token = Some(token.clone());

                            let snapshot = Self::build_snapshot(
                                &app_state,
                                &project_id,
                                Some(QueueState::Running),
                            );
                            let _ = handle_clone.emit("queue-snapshot", &snapshot);

                            let worker_count = 2;
                            for _ in 0..worker_count {
                                let app_state_runner = Arc::clone(&app_state);
                                let handle_runner = handle_clone.clone();
                                let project_id_runner = project_id.clone();
                                let tx_runner = tx_for_worker.clone();
                                let token_runner = token.clone();

                                tauri::async_runtime::spawn(async move {
                                    Self::run_worker_loop(
                                        project_id_runner,
                                        app_state_runner,
                                        handle_runner,
                                        token_runner,
                                        tx_runner,
                                    )
                                    .await;
                                });
                            }
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

                            let snapshot = Self::build_snapshot(
                                &app_state,
                                &project_id,
                                Some(QueueState::Cancelled),
                            );
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
                    QueueCommand::WorkerFinished {
                        project_id,
                        outcome,
                    } => {
                        if current_project_id.as_deref() == Some(&project_id) && !is_paused {
                            current_project_id = None;
                            cancel_token = None;
                            let final_state = match outcome {
                                WorkerOutcome::Completed => QueueState::Completed,
                                WorkerOutcome::Cancelled => QueueState::Cancelled,
                                WorkerOutcome::Failed(_) => QueueState::Failed,
                            };

                            let snapshot =
                                Self::build_snapshot(&app_state, &project_id, Some(final_state));
                            let _ = handle_clone.emit("queue-snapshot", &snapshot);
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn enqueue_project(&self, project_id: String) -> Result<QueueSnapshot, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(QueueCommand::EnqueueProject {
                project_id,
                reply: reply_tx,
            })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx
            .await
            .map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn pause_project(&self, project_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(QueueCommand::PauseProject {
                project_id,
                reply: reply_tx,
            })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx
            .await
            .map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn resume_project(&self, project_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(QueueCommand::ResumeProject {
                project_id,
                reply: reply_tx,
            })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx
            .await
            .map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn cancel_project(&self, project_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(QueueCommand::CancelProject {
                project_id,
                reply: reply_tx,
            })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx
            .await
            .map_err(|_| "Actor dropped reply".to_string())?
    }

    pub async fn get_queue_snapshot(&self, project_id: String) -> Result<QueueSnapshot, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(QueueCommand::GetSnapshot {
                project_id,
                reply: reply_tx,
            })
            .await
            .map_err(|e| format!("Queue actor offline: {}", e))?;
        reply_rx
            .await
            .map_err(|_| "Actor dropped reply".to_string())?
    }

    fn build_snapshot(
        app_state: &AppState,
        project_id: &str,
        override_state: Option<QueueState>,
    ) -> QueueSnapshot {
        let db = match &app_state.db {
            Some(d) => d,
            None => {
                return QueueSnapshot {
                    project_id: project_id.to_string(),
                    queue_state: override_state.unwrap_or(QueueState::Idle),
                    total_segments: 0,
                    completed_segments: 0,
                    failed_segments: 0,
                    pending_segments: 0,
                    snapshot_revision: 0,
                }
            }
        };

        let segs = ProjectRepository::get_segments_for_project(db, project_id).unwrap_or_default();
        let total = segs.len();
        use crate::models::segment::SegmentStatus;
        let completed = segs
            .iter()
            .filter(|s| s.status == SegmentStatus::Success || s.status == SegmentStatus::Approved)
            .count();
        let failed = segs
            .iter()
            .filter(|s| s.status == SegmentStatus::Failed)
            .count();
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
        crate::storage::project_repo::ProjectRepository::get_segment_counts(db, project_id)
            .unwrap_or((0, 0, 0))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_progress_event(
        handle: &tauri::AppHandle,
        stream_id: &str,
        sequence: u64,
        project_id: &str,
        segment_id: Option<String>,
        position: usize,
        total_segments: usize,
        completed_segments: usize,
        status: &str,
        revision: u64,
    ) {
        let _ = handle.emit(
            "queue-progress",
            QueueProgressEvent {
                stream_id: stream_id.to_string(),
                sequence,
                project_id: project_id.to_string(),
                segment_id,
                position,
                total_segments,
                completed_segments,
                status: status.to_string(),
                revision,
                error_message: None,
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn save_synthesis_output(
        app_state: &AppState,
        project_id: &str,
        task: &crate::storage::project_repo::SegmentRecord,
        worker_id: &str,
        pcm_bytes: &[u8],
        latency_ms: u64,
        char_count: usize,
        handle: &tauri::AppHandle,
        sequence: &mut u64,
    ) -> Result<bool, String> {
        app_state
            .total_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        app_state
            .total_chars
            .fetch_add(char_count as u64, std::sync::atomic::Ordering::Relaxed);
        app_state
            .total_latency_ms
            .fetch_add(latency_ms, std::sync::atomic::Ordering::Relaxed);
        if let Some(ref db_instance) = app_state.db {
            let _ = db_instance.record_quota_metric(char_count, false, latency_ms);
        }

        let fp = task.fingerprint.as_deref().unwrap_or("legacy_fp");
        let cached_wav_path = app_state.output_dir.join(format!("{}.wav", fp));
        let wav_path_str = cached_wav_path.to_string_lossy().to_string();

        match write_pcm_to_wav_atomic(pcm_bytes, &wav_path_str) {
            Ok(dur_ms) => {
                let file_size = std::fs::metadata(&cached_wav_path)
                    .map(|m| m.len())
                    .unwrap_or(0);

                let db = match &app_state.db {
                    Some(d) => d,
                    None => return Err("DB offline".to_string()),
                };

                let commit_res = ProjectRepository::commit_task_result(
                    db,
                    &task.id,
                    worker_id,
                    Some(fp),
                    crate::models::segment::SegmentStatus::Success,
                    Some(&wav_path_str),
                    file_size,
                    dur_ms,
                    None,
                    None,
                    None,
                );

                match commit_res {
                    Ok(true) => {
                        *sequence += 1;
                        let (t_segs, c_segs, _) = Self::get_segment_counts(app_state, project_id);
                        Self::emit_progress_event(
                            handle,
                            &format!("stream-{}", project_id),
                            *sequence,
                            project_id,
                            Some(task.id.clone()),
                            task.position,
                            t_segs,
                            c_segs,
                            "success",
                            task.state_revision + 1,
                        );
                        Ok(true)
                    }
                    Ok(false) => {
                        warn!(
                            "Segment {} was modified during processing, discarding stale audio",
                            task.id
                        );
                        let _ = std::fs::remove_file(&wav_path_str);
                        Ok(false)
                    }
                    Err(e) => {
                        error!(
                            "Failed to commit task result for segment {}: {}",
                            task.id, e
                        );
                        Err(e.to_string())
                    }
                }
            }
            Err(e) => {
                error!(
                    "Atomic WAV write/validation failed for task {}: {}",
                    task.id, e
                );
                let db = match &app_state.db {
                    Some(d) => d,
                    None => return Err("DB offline".to_string()),
                };
                let _ = ProjectRepository::commit_task_result(
                    db,
                    &task.id,
                    worker_id,
                    None,
                    crate::models::segment::SegmentStatus::Failed,
                    None,
                    0,
                    0,
                    None,
                    Some("AUDIO_CORRUPT"),
                    Some(&e.to_string()),
                );
                Err(e.to_string())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_synthesis_error(
        app_state: &AppState,
        project_id: &str,
        task: &crate::storage::project_repo::SegmentRecord,
        worker_id: &str,
        err: &crate::api::provider::ProviderError,
        latency_ms: u64,
        api_key: &str,
        consecutive_quota_errors: &mut usize,
        handle: &tauri::AppHandle,
    ) -> bool {
        let is_unauthorized = err.code.contains("Unauthorized") || err.code.contains("Forbidden");
        let is_daily_quota = err.code.contains("RateLimitedDaily") || err.message.contains("quota");
        let is_rate_limit = err.is_rate_limit || is_daily_quota;

        let (friendly_msg, err_code_str) = if is_unauthorized {
            (
                "Gemini API Key không hợp lệ hoặc đã bị vô hiệu hóa. Vui lòng kiểm tra lại cấu hình Key.".to_string(),
                "AUTH_INVALID",
            )
        } else if is_daily_quota {
            (
                "Hạn ngạch sử dụng Google API trong ngày đã hết. Hàng đợi đã tạm dừng.".to_string(),
                "DAILY_QUOTA_EXHAUSTED",
            )
        } else if err.is_rate_limit {
            (
                format!(
                    "Đã đạt giới hạn tốc độ yêu cầu (Rate Limit 429). Đang chờ thử lại sau {}s...",
                    err.retry_after_secs.unwrap_or(10)
                ),
                "RATE_LIMITED",
            )
        } else if err.code.contains("ContentFiltered") {
            (
                "Đoạn văn bản bị từ chối bởi bộ lọc an toàn của Google.".to_string(),
                "CONTENT_FILTERED",
            )
        } else {
            (
                format!("Lỗi kết nối API Google: {}", err.message),
                "INTERNAL_ERROR",
            )
        };

        warn!(
            "Task {} attempt {} failed: {} ({})",
            task.id,
            task.attempt_count + 1,
            friendly_msg,
            err_code_str
        );

        if is_rate_limit {
            app_state
                .rate_limit_hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if let Some(ref db_instance) = app_state.db {
                let _ = db_instance.record_quota_metric(0, true, latency_ms);
            }
        }

        let db = match &app_state.db {
            Some(d) => d,
            None => return false,
        };

        if is_daily_quota {
            app_state.credentials.mark_key_daily_exhausted(api_key);
            *consecutive_quota_errors += 1;
            let key_count = app_state.credentials.key_count().max(1);

            let _ = ProjectRepository::commit_task_result(
                db,
                &task.id,
                worker_id,
                None,
                crate::models::segment::SegmentStatus::RetryWait,
                None,
                0,
                0,
                Some(Utc::now().timestamp_millis()),
                Some(err_code_str),
                Some(&friendly_msg),
            );

            if *consecutive_quota_errors < key_count {
                warn!("Key daily quota exhausted, switching to another key...");
                return true;
            } else {
                warn!(
                    "All {} keys exhausted daily quota. Pausing worker for 60s.",
                    key_count
                );
                let _ = handle.emit(
                    "queue-paused",
                    serde_json::json!({
                        "project_id": project_id,
                        "reason": "quota_exhausted",
                        "message": friendly_msg
                    }),
                );
                sleep(Duration::from_secs(60)).await;
                return true;
            }
        }

        *consecutive_quota_errors = 0;
        let is_retryable = if is_unauthorized || err.code.contains("ContentFiltered") {
            false
        } else if is_rate_limit {
            let cooldown = err.retry_after_secs.unwrap_or(30);
            app_state.credentials.mark_key_cooldown(api_key, cooldown);
            true
        } else {
            true
        };

        if is_retryable && task.attempt_count < 3 {
            let base_delay = if err.is_rate_limit {
                err.retry_after_secs.unwrap_or(10)
            } else {
                u64::pow(2, task.attempt_count + 1)
            };
            let jitter_ms = (Utc::now().timestamp_subsec_millis() as u64 % 1000) + 500;
            let now_ms = Utc::now().timestamp_millis();
            let next_retry = now_ms + (base_delay * 1000 + jitter_ms) as i64;

            let _ = ProjectRepository::commit_task_result(
                db,
                &task.id,
                worker_id,
                None,
                crate::models::segment::SegmentStatus::RetryWait,
                None,
                0,
                0,
                Some(next_retry),
                Some(err_code_str),
                Some(&friendly_msg),
            );
        } else {
            let _ = ProjectRepository::commit_task_result(
                db,
                &task.id,
                worker_id,
                None,
                crate::models::segment::SegmentStatus::Failed,
                None,
                0,
                0,
                None,
                Some(err_code_str),
                Some(&friendly_msg),
            );
        }

        false
    }

    async fn run_worker_loop(
        project_id: String,
        app_state: Arc<AppState>,
        handle: tauri::AppHandle,
        cancel_token: CancellationToken,
        tx: mpsc::Sender<QueueCommand>,
    ) {
        let worker_id = format!("worker_{}", Uuid::new_v4().simple());
        let lease_duration_secs = 45u64;
        let mut sequence: u64 = 0;
        let final_outcome;
        let mut consecutive_quota_errors: usize = 0;

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

            let task = match ProjectRepository::claim_next_task(
                db,
                &project_id,
                &worker_id,
                lease_duration_secs,
            ) {
                Ok(Some(t)) => t,
                Ok(None) => {
                    if let Ok(Some(delay_ms)) =
                        ProjectRepository::get_next_retry_delay_ms(db, &project_id)
                    {
                        let wait_time = delay_ms.clamp(100, 2000);
                        info!(
                            "No immediate task ready for project {}, sleeping {}ms...",
                            project_id, wait_time
                        );
                        sleep(Duration::from_millis(wait_time)).await;
                        continue;
                    }
                    info!(
                        "No more queued tasks for project {}. Worker exiting loop.",
                        project_id
                    );
                    let (total, comp, fail) = Self::get_segment_counts(&app_state, &project_id);
                    if fail > 0 && comp + fail >= total {
                        final_outcome = WorkerOutcome::Failed(format!(
                            "Queue finished with {} failed segments",
                            fail
                        ));
                    } else {
                        final_outcome = WorkerOutcome::Completed;
                    }
                    break;
                }
                Err(e) => {
                    error!("Error claiming task for project {}: {}", project_id, e);
                    sleep(Duration::from_millis(ERROR_RETRY_DELAY_MS)).await;
                    continue;
                }
            };

            sequence += 1;
            let (total_segs, completed_segs, _) = Self::get_segment_counts(&app_state, &project_id);
            Self::emit_progress_event(
                &handle,
                &format!("stream-{}", project_id),
                sequence,
                &project_id,
                Some(task.id.clone()),
                task.position,
                total_segs,
                completed_segs,
                "processing",
                task.state_revision,
            );

            let api_key = match app_state.credentials.get_key() {
                Some(k) => k,
                None => {
                    let _ = ProjectRepository::commit_task_result(
                        db,
                        &task.id,
                        &worker_id,
                        None,
                        SegmentStatus::Failed,
                        None,
                        0,
                        0,
                        None,
                        Some("AUTH_INVALID"),
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
                    SegmentStatus::Queued,
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

            let proj_record = ProjectRepository::get_project_by_id(db, &project_id)
                .ok()
                .flatten();
            let model_name = proj_record
                .map(|p| p.model)
                .unwrap_or_else(|| "gemini-3.1-flash-tts-preview".to_string());

            let credentials = crate::api::provider::ProviderCredentials {
                api_key: api_key.clone(),
                project_id: None,
            };
            let config = crate::api::provider::SynthesisConfig {
                model: model_name,
                voice: task.voice.clone().unwrap_or_else(|| "Kore".to_string()),
                speaking_rate: 1.0,
                pitch_shift: 0.0,
                volume_gain_db: 0.0,
            };
            let provider = crate::api::provider::GeminiProvider::new();
            let start_inst = std::time::Instant::now();
            let pcm_result = provider
                .synthesize(&credentials, &task.prompt, &config)
                .await;
            let latency_ms = start_inst.elapsed().as_millis() as u64;
            let char_count = task.text.chars().count();

            match pcm_result {
                Ok(output) => {
                    let pcm_bytes = output.audio_bytes;
                    if let Ok(true) = Self::save_synthesis_output(
                        &app_state,
                        &project_id,
                        &task,
                        &worker_id,
                        &pcm_bytes,
                        latency_ms,
                        char_count,
                        &handle,
                        &mut sequence,
                    ) {
                        consecutive_quota_errors = 0;
                    }
                }
                Err(err) => {
                    if Self::handle_synthesis_error(
                        &app_state,
                        &project_id,
                        &task,
                        &worker_id,
                        &err,
                        latency_ms,
                        &api_key,
                        &mut consecutive_quota_errors,
                        &handle,
                    )
                    .await
                    {
                        continue;
                    }
                }
            }

            let key_count = app_state.credentials.key_count().max(1) as u64;
            let rate_delay_ms = (4100u64 / key_count).max(MIN_RATE_DELAY_MS);
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
        let _ = tx
            .send(QueueCommand::WorkerFinished {
                project_id,
                outcome: final_outcome,
            })
            .await;
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
        assert!(is_error_retryable(&ApiError::NetworkError(
            "Timeout".to_string()
        )));
        assert!(is_error_retryable(&ApiError::ApiServerError(
            500,
            "Internal Server Error".to_string()
        )));
        assert!(is_error_retryable(&ApiError::ApiServerError(
            503,
            "Service Unavailable".to_string()
        )));

        assert!(!is_error_retryable(&ApiError::Unauthorized));
        assert!(!is_error_retryable(&ApiError::MissingAudio));
        assert!(!is_error_retryable(&ApiError::ApiServerError(
            400,
            "Bad Request".to_string()
        )));
        assert!(!is_error_retryable(&ApiError::ApiServerError(
            403,
            "Forbidden".to_string()
        )));
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
