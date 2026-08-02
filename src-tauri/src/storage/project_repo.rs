use crate::storage::db::DatabaseManager;
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

const MS_PER_SEC: u64 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub source_text: String,
    pub model: String,
    pub voice: String,
    pub preset: String,
    pub pacing: String,
    pub pronunciation_notes: Option<String>,
    pub output_directory: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub use crate::models::segment::{ReviewStatus, SegmentStatus, SynthesisStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRecord {
    pub id: String,
    pub project_id: String,
    pub position: usize,
    pub text: String,
    pub prompt: String,
    pub status: SegmentStatus,
    pub attempts: u32,
    pub audio_path: Option<String>,
    pub duration_ms: u64,
    pub error_code: Option<u16>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub fingerprint: Option<String>,
    pub output_fingerprint: Option<String>,
    pub attempt_count: u32,
    pub next_retry_at: Option<i64>,
    pub queued_at: Option<i64>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<i64>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub cancel_requested: bool,
    pub state_revision: u64,
    pub output_size: u64,
    pub voice: Option<String>,
    pub synthesis_status: Option<SynthesisStatus>,
    pub review_status: Option<ReviewStatus>,
    pub reviewed_output_fingerprint: Option<String>,
}

const SEGMENT_SELECT: &str = "SELECT id, project_id, position, text, prompt, status, attempts, audio_path, duration_ms, \
        error_code, error_message, created_at, updated_at, fingerprint, output_fingerprint, \
        attempt_count, next_retry_at, queued_at, started_at, finished_at, lease_owner, \
        lease_expires_at, last_error_code, last_error_message, cancel_requested, state_revision, output_size, voice, \
        synthesis_status, review_status, reviewed_output_fingerprint FROM segments";

fn map_segment_row(row: &rusqlite::Row) -> rusqlite::Result<SegmentRecord> {
    let pos: i64 = row.get(2)?;
    let dur: i64 = row.get(8)?;
    let cancel_req: i32 = row.get(24).unwrap_or(0);
    let rev: i64 = row.get(25).unwrap_or(0);
    let size: i64 = row.get(26).unwrap_or(0);
    Ok(SegmentRecord {
        id: row.get(0)?,
        project_id: row.get(1)?,
        position: pos as usize,
        text: row.get(3)?,
        prompt: row.get(4)?,
        status: row.get(5)?,
        attempts: row.get(6)?,
        audio_path: row.get(7)?,
        duration_ms: dur as u64,
        error_code: row.get(9)?,
        error_message: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        fingerprint: row.get(13)?,
        output_fingerprint: row.get(14)?,
        attempt_count: row.get(15).unwrap_or(0),
        next_retry_at: row.get(16)?,
        queued_at: row.get(17)?,
        started_at: row.get(18)?,
        finished_at: row.get(19)?,
        lease_owner: row.get(20)?,
        lease_expires_at: row.get(21)?,
        last_error_code: row.get(22)?,
        last_error_message: row.get(23)?,
        cancel_requested: cancel_req != 0,
        state_revision: rev as u64,
        output_size: size as u64,
        voice: row.get(27)?,
        synthesis_status: row.get(28)?,
        review_status: row.get(29)?,
        reviewed_output_fingerprint: row.get(30)?,
    })
}

pub struct ProjectRepository;

impl ProjectRepository {
    pub fn create_project(db: &DatabaseManager, proj: &ProjectRecord) -> Result<(), String> {
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, source_text, model, voice, preset, pacing, pronunciation_notes, output_directory, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    proj.id,
                    proj.name,
                    proj.source_text,
                    proj.model,
                    proj.voice,
                    proj.preset,
                    proj.pacing,
                    proj.pronunciation_notes,
                    proj.output_directory,
                    proj.status,
                    proj.created_at,
                    proj.updated_at
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_projects(db: &DatabaseManager) -> Result<Vec<ProjectRecord>, String> {
        db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, source_text, model, voice, preset, pacing, pronunciation_notes, output_directory, status, created_at, updated_at FROM projects ORDER BY updated_at DESC",
            )?;
            let proj_iter = stmt.query_map([], |row| {
                Ok(ProjectRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_text: row.get(2)?,
                    model: row.get(3)?,
                    voice: row.get(4)?,
                    preset: row.get(5)?,
                    pacing: row.get(6)?,
                    pronunciation_notes: row.get(7)?,
                    output_directory: row.get(8)?,
                    status: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })?;

            let mut list = Vec::new();
            for p in proj_iter.flatten() {
                list.push(p);
            }
            Ok(list)
        })
    }

    pub fn get_project_by_id(
        db: &DatabaseManager,
        id: &str,
    ) -> Result<Option<ProjectRecord>, String> {
        db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, source_text, model, voice, preset, pacing, pronunciation_notes, output_directory, status, created_at, updated_at FROM projects WHERE id = ?1",
            )?;
            let proj = stmt.query_row(params![id], |row| {
                Ok(ProjectRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_text: row.get(2)?,
                    model: row.get(3)?,
                    voice: row.get(4)?,
                    preset: row.get(5)?,
                    pacing: row.get(6)?,
                    pronunciation_notes: row.get(7)?,
                    output_directory: row.get(8)?,
                    status: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            }).optional()?;
            Ok(proj)
        })
    }

    pub fn get_project(
        db: &DatabaseManager,
        project_id: &str,
    ) -> Result<Option<ProjectRecord>, String> {
        Self::get_project_by_id(db, project_id)
    }

    pub fn delete_project(db: &DatabaseManager, project_id: &str) -> Result<(), String> {
        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            // Collect audio paths before deleting records
            let paths: Vec<String> = {
                let mut stmt = tx.prepare("SELECT audio_path FROM segments WHERE project_id = ?1 AND audio_path IS NOT NULL")?;
                let res: Vec<String> = stmt.query_map(params![project_id], |row| row.get(0))?
                    .filter_map(|r| r.ok())
                    .collect();
                res
            };

            tx.execute("DELETE FROM segments WHERE project_id = ?1", params![project_id])?;
            tx.execute("DELETE FROM projects WHERE id = ?1", params![project_id])?;
            tx.commit()?;

            // Clean up audio files after successful DB delete
            for path in paths {
                let _ = std::fs::remove_file(&path);
            }

            Ok(())
        })
    }

    pub fn delete_projects_batch(
        db: &DatabaseManager,
        project_ids: &[String],
    ) -> Result<(), String> {
        for id in project_ids {
            let _ = Self::delete_project(db, id);
        }
        Ok(())
    }

    pub fn delete_segment(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
    ) -> Result<(), String> {
        Self::delete_segments_batch(db, project_id, &[segment_id.to_string()])
    }

    pub fn delete_segments_batch(
        db: &DatabaseManager,
        project_id: &str,
        segment_ids: &[String],
    ) -> Result<(), String> {
        if segment_ids.is_empty() {
            return Ok(());
        }

        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            for seg_id in segment_ids {
                let path: Option<String> = tx
                    .query_row(
                        "SELECT audio_path FROM segments WHERE project_id = ?1 AND id = ?2",
                        params![project_id, seg_id],
                        |row| row.get(0),
                    )
                    .ok()
                    .flatten();

                tx.execute(
                    "DELETE FROM segments WHERE project_id = ?1 AND id = ?2",
                    params![project_id, seg_id],
                )?;

                if let Some(p) = path {
                    let _ = std::fs::remove_file(p);
                }
            }

            tx.execute(
                "UPDATE segments
                 SET position = (
                     SELECT COUNT(*)
                     FROM segments AS s2
                     WHERE s2.project_id = segments.project_id
                       AND s2.position <= segments.position
                 )
                 WHERE project_id = ?1",
                params![project_id],
            )?;

            tx.commit()?;
            Ok(())
        })
    }

    pub fn insert_segment_at(
        db: &DatabaseManager,
        project_id: &str,
        position: usize,
        text: &str,
    ) -> Result<(), String> {
        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            let preset: String = tx
                .query_row(
                    "SELECT preset FROM projects WHERE id = ?1",
                    params![project_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| "Đọc tự nhiên".to_string());

            tx.execute(
                "UPDATE segments SET position = position + 1 WHERE project_id = ?1 AND position >= ?2",
                params![project_id, position as i64],
            )?;

            let now = Utc::now().to_rfc3339();
            let seg_id = uuid::Uuid::new_v4().to_string();

            tx.execute(
                "INSERT INTO segments (
                    id, project_id, position, text, prompt, status, attempts, duration_ms, created_at, updated_at, attempt_count, cancel_requested, state_revision, output_size
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, ?7, ?7, 0, 0, 1, 0)",
                params![seg_id, project_id, position as i64, text, preset, "pending", now],
            )?;

            tx.commit()?;
            Ok(())
        })
    }

    pub fn swap_segment_positions(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
        direction: &str,
    ) -> Result<(), String> {
        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let current_pos: usize = tx.query_row(
                "SELECT position FROM segments WHERE project_id = ?1 AND id = ?2",
                params![project_id, segment_id],
                |row| row.get(0),
            )?;

            let target_pos = if direction == "up" {
                if current_pos <= 1 {
                    return Ok(());
                }
                current_pos - 1
            } else {
                let max_pos: usize = tx.query_row(
                    "SELECT COALESCE(MAX(position), 1) FROM segments WHERE project_id = ?1",
                    params![project_id],
                    |row| row.get(0),
                )?;
                if current_pos >= max_pos {
                    return Ok(());
                }
                current_pos + 1
            };

            let other_id: String = tx.query_row(
                "SELECT id FROM segments WHERE project_id = ?1 AND position = ?2",
                params![project_id, target_pos as i64],
                |row| row.get(0),
            )?;

            tx.execute(
                "UPDATE segments SET position = ?1 WHERE id = ?2",
                params![target_pos as i64, segment_id],
            )?;
            tx.execute(
                "UPDATE segments SET position = ?1 WHERE id = ?2",
                params![current_pos as i64, other_id],
            )?;

            tx.commit()?;
            Ok(())
        })
    }

    pub fn insert_segments(db: &DatabaseManager, segments: &[SegmentRecord]) -> Result<(), String> {
        if segments.is_empty() {
            return Ok(());
        }

        const BATCH_SIZE: usize = 500;
        for chunk in segments.chunks(BATCH_SIZE) {
            db.with_conn_mut(|conn| {
                let tx = conn.transaction()?;
                {
                    let mut stmt = tx.prepare_cached(
                        "INSERT INTO segments (
                            id, project_id, position, text, prompt, status, attempts, audio_path, duration_ms,
                            error_code, error_message, created_at, updated_at, fingerprint, output_fingerprint,
                            attempt_count, next_retry_at, queued_at, started_at, finished_at, lease_owner,
                            lease_expires_at, last_error_code, last_error_message, cancel_requested, state_revision, output_size, voice
                        ) VALUES (
                            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28
                        )"
                    )?;

                    for s in chunk {
                        stmt.execute(params![
                            s.id,
                            s.project_id,
                            s.position as i64,
                            s.text,
                            s.prompt,
                            s.status,
                            s.attempts,
                            s.audio_path,
                            s.duration_ms as i64,
                            s.error_code,
                            s.error_message,
                            s.created_at,
                            s.updated_at,
                            s.fingerprint,
                            s.output_fingerprint,
                            s.attempt_count,
                            s.next_retry_at,
                            s.queued_at,
                            s.started_at,
                            s.finished_at,
                            s.lease_owner,
                            s.lease_expires_at,
                            s.last_error_code,
                            s.last_error_message,
                            s.cancel_requested as i32,
                            s.state_revision as i64,
                            s.output_size as i64,
                            s.voice,
                        ])?;
                    }
                }
                tx.commit()?;
                Ok(())
            })?;
        }

        Ok(())
    }

    pub fn get_segment_counts(
        db: &DatabaseManager,
        project_id: &str,
    ) -> Result<(usize, usize, usize), String> {
        db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT 
                    COUNT(*) as total,
                    SUM(CASE WHEN status = 'success' OR status = 'approved' THEN 1 ELSE 0 END) as completed,
                    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
                 FROM segments 
                 WHERE project_id = ?1"
            )?;

            let (total, completed, failed) = stmt.query_row(params![project_id], |row| {
                let total: i64 = row.get(0).unwrap_or(0);
                let completed: i64 = row.get(1).unwrap_or(0);
                let failed: i64 = row.get(2).unwrap_or(0);
                Ok((total as usize, completed as usize, failed as usize))
            })?;

            Ok((total, completed, failed))
        })
    }

    pub fn get_segments_for_project(
        db: &DatabaseManager,
        project_id: &str,
    ) -> Result<Vec<SegmentRecord>, String> {
        db.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE project_id = ?1 ORDER BY position ASC",
                SEGMENT_SELECT
            ))?;

            let seg_iter = stmt.query_map(params![project_id], map_segment_row)?;

            let mut list = Vec::new();
            for s in seg_iter.flatten() {
                list.push(s);
            }
            Ok(list)
        })
    }

    pub fn mark_project_segments_queued(
        db: &DatabaseManager,
        project_id: &str,
    ) -> Result<usize, String> {
        let now_ms = Utc::now().timestamp_millis();
        db.with_conn(|conn| {
            let count = conn.execute(
                "UPDATE segments
                 SET status = 'queued',
                     queued_at = ?1,
                     state_revision = state_revision + 1
                 WHERE project_id = ?2 AND status IN ('pending', 'failed', 'retry_wait', 'stale')",
                params![now_ms, project_id],
            )?;
            Ok(count)
        })
    }

    pub fn claim_next_task(
        db: &DatabaseManager,
        project_id: &str,
        worker_id: &str,
        lease_duration_secs: u64,
    ) -> Result<Option<SegmentRecord>, String> {
        let now_ms = Utc::now().timestamp_millis();
        let lease_expires_at = now_ms + (lease_duration_secs * MS_PER_SEC) as i64;

        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            // Find candidate segment
            let candidate_id: Option<String> = tx
                .query_row(
                    "SELECT id FROM segments
                 WHERE project_id = ?1
                   AND (
                       status = 'queued'
                       OR (status = 'retry_wait' AND (next_retry_at IS NULL OR next_retry_at <= ?2))
                   )
                   AND cancel_requested = 0
                 ORDER BY position ASC
                 LIMIT 1",
                    params![project_id, now_ms],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(seg_id) = candidate_id {
                let updated = tx.execute(
                    "UPDATE segments
                     SET status = 'processing',
                         started_at = ?1,
                         lease_owner = ?2,
                         lease_expires_at = ?3,
                         attempts = attempts + 1,
                         attempt_count = attempt_count + 1,
                         state_revision = state_revision + 1,
                         updated_at = ?4
                     WHERE id = ?5 AND status IN ('queued', 'retry_wait')",
                    params![
                        now_ms,
                        worker_id,
                        lease_expires_at,
                        Utc::now().to_rfc3339(),
                        seg_id
                    ],
                )?;

                if updated > 0 {
                    let seg = {
                        let mut stmt = tx.prepare(&format!("{} WHERE id = ?1", SEGMENT_SELECT))?;

                        stmt.query_row(params![seg_id], map_segment_row)?
                    };

                    tx.commit()?;
                    return Ok(Some(seg));
                }
            }

            tx.commit()?;
            Ok(None)
        })
    }

    pub fn get_next_retry_delay_ms(
        db: &DatabaseManager,
        project_id: &str,
    ) -> Result<Option<u64>, String> {
        let now_ms = Utc::now().timestamp_millis();
        db.with_conn(|conn| {
            let next_retry: Option<i64> = conn
                .query_row(
                    "SELECT MIN(next_retry_at) FROM segments
                 WHERE project_id = ?1
                   AND status = 'retry_wait'
                   AND cancel_requested = 0",
                    params![project_id],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(retry_time) = next_retry {
                if retry_time > now_ms {
                    let diff = (retry_time - now_ms) as u64;
                    return Ok(Some(diff));
                } else {
                    return Ok(Some(0));
                }
            }

            Ok(None)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit_task_result(
        db: &DatabaseManager,
        segment_id: &str,
        worker_id: &str,
        fingerprint: Option<&str>,
        status: SegmentStatus,
        output_path: Option<&str>,
        output_bytes_len: u64,
        duration_ms: u64,
        next_retry_at: Option<i64>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<bool, String> {
        let now_ms = Utc::now().timestamp_millis();
        let now_iso = Utc::now().to_rfc3339();

        db.with_conn(|conn| {
            let updated = conn.execute(
                "UPDATE segments
                 SET status = ?1,
                     output_fingerprint = CASE WHEN ?1 = 'success' THEN ?2 ELSE output_fingerprint END,
                     audio_path = CASE WHEN ?1 = 'success' THEN ?3 ELSE audio_path END,
                     output_size = CASE WHEN ?1 = 'success' THEN ?4 ELSE output_size END,
                     duration_ms = CASE WHEN ?1 = 'success' THEN ?5 ELSE duration_ms END,
                     finished_at = CASE WHEN ?1 IN ('success', 'failed') THEN ?6 ELSE finished_at END,
                     next_retry_at = ?7,
                     last_error_code = ?8,
                     last_error_message = ?9,
                     error_message = ?9,
                     lease_owner = NULL,
                     lease_expires_at = NULL,
                     state_revision = state_revision + 1,
                     updated_at = ?10
                 WHERE id = ?11
                   AND status = 'processing'
                   AND lease_owner = ?12",
                params![
                    status.to_string(),
                    fingerprint,
                    output_path,
                    output_bytes_len as i64,
                    duration_ms as i64,
                    now_ms,
                    next_retry_at,
                    error_code,
                    error_message,
                    now_iso,
                    segment_id,
                    worker_id
                ],
            )?;
            Ok(updated > 0)
        })
    }

    pub fn requeue_segment(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        db.with_conn(|conn| {
            let rows_affected = conn.execute(
                "UPDATE segments SET status = 'pending', attempts = 0, attempt_count = 0, error_code = NULL, error_message = NULL, last_error_code = NULL, last_error_message = NULL, state_revision = state_revision + 1, updated_at = ?1
                 WHERE id = ?2 AND project_id = ?3",
                params![now, segment_id, project_id],
            )?;
            if rows_affected == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_segment_status(
        db: &DatabaseManager,
        segment_id: &str,
        status: SegmentStatus,
        attempts: u32,
        audio_path: Option<&str>,
        duration_ms: u64,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        db.with_conn(|conn| {
            conn.execute(
                "UPDATE segments SET status = ?1, attempts = ?2, audio_path = ?3, duration_ms = ?4, last_error_code = ?5, error_message = ?6, updated_at = ?7, state_revision = state_revision + 1 WHERE id = ?8",
                params![
                    status.to_string(),
                    attempts,
                    audio_path,
                    duration_ms as i64,
                    error_code,
                    error_message,
                    now,
                    segment_id
                ],
            )?;
            Ok(())
        })
    }

    pub fn update_segment_text(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
        text: &str,
        prompt: &str,
        fingerprint: &str,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let old_audio_path: Option<String> = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT audio_path FROM segments WHERE id = ?1 AND project_id = ?2",
                    params![segment_id, project_id],
                    |row| row.get(0),
                )
                .optional()
                .map(|opt| opt.flatten())
            })
            .unwrap_or(None);

        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let rows_affected = tx.execute(
                "UPDATE segments
                 SET text = ?1, prompt = ?2, fingerprint = ?3, status = 'pending', audio_path = NULL, duration_ms = 0, state_revision = state_revision + 1, updated_at = ?4
                 WHERE id = ?5 AND project_id = ?6",
                params![text, prompt, fingerprint, now, segment_id, project_id],
            )?;

            if rows_affected == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }

            tx.execute(
                "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
                params![now, project_id],
            )?;

            tx.commit()?;

            // Cleanup orphaned WAV file after successful commit
            if let Some(ref path_str) = old_audio_path {
                let path = std::path::Path::new(path_str);
                if path.exists() {
                    match std::fs::remove_file(path) {
                        Ok(_) => info!("Cleaned up orphaned audio: {}", path_str),
                        Err(e) => warn!("Failed to cleanup orphaned audio {}: {}", path_str, e),
                    }
                }
            }

            Ok(())
        })
    }

    pub fn update_text(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
        text: &str,
        prompt: &str,
        fingerprint: &str,
    ) -> Result<(), String> {
        Self::update_segment_text(db, project_id, segment_id, text, prompt, fingerprint)
    }

    pub fn update_voice(
        db: &DatabaseManager,
        project_id: &str,
        voice_id: &str,
    ) -> Result<(), String> {
        Self::update_project_voice(db, project_id, voice_id)
    }

    pub fn update_project_voice(
        db: &DatabaseManager,
        project_id: &str,
        voice_id: &str,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        db.with_conn(|conn| {
            let rows_affected = conn.execute(
                "UPDATE projects SET voice = ?1, updated_at = ?2 WHERE id = ?3",
                params![voice_id, now, project_id],
            )?;

            if rows_affected == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }

    pub fn update_segment_voice(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
        voice: Option<&str>,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        db.with_conn(|conn| {
            conn.execute(
                "UPDATE segments SET voice = ?1, updated_at = ?2, state_revision = state_revision + 1 WHERE project_id = ?3 AND id = ?4",
                params![voice, now, project_id, segment_id],
            )?;
            Ok(())
        })
    }

    pub fn split_segment(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
        split_index: usize,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();

        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            let seg: SegmentRecord = {
                let mut stmt = tx.prepare(
                    &format!("{} WHERE id = ?1 AND project_id = ?2", SEGMENT_SELECT),
                )?;

                stmt.query_row(params![segment_id, project_id], map_segment_row)?
            };

            let (voice, preset) = {
                let res = tx.query_row(
                    "SELECT voice, preset FROM projects WHERE id = ?1",
                    params![project_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                );
                res.unwrap_or_else(|_| ("Kore".to_string(), "Tự nhiên".to_string()))
            };

            let text = &seg.text;
            let byte_pos = if split_index <= text.len() && text.is_char_boundary(split_index) {
                split_index
            } else {
                text.char_indices().map(|(i, _)| i).nth(split_index).unwrap_or(text.len())
            };

            let text1 = text[..byte_pos].trim().to_string();
            let text2 = text[byte_pos..].trim().to_string();

            if text1.is_empty() || text2.is_empty() {
                return Err(rusqlite::Error::InvalidParameterName("Split index produces empty segment text".to_string()));
            }

            let prompt_opts = crate::text::prompt_builder::PromptStyleOptions {
                style_preset: preset,
                pacing: "Bình thường".to_string(),
                pronunciation_notes: None,
            };

            let prompt1 = crate::text::prompt_builder::build_tts_prompt(&text1, &prompt_opts);
            let prompt2 = crate::text::prompt_builder::build_tts_prompt(&text2, &prompt_opts);

            let fp1 = crate::text::fingerprint::compute_segment_fingerprint(&crate::text::fingerprint::SegmentFingerprintInput {
                text: &text1,
                voice: &voice,
                model: "gemini-3.1-flash-tts-preview",
                speaking_rate: 1.0,
                pitch_shift: 0.0,
                volume_gain_db: 0.0,
                sample_rate_hz: 24000,
            });

            let fp2 = crate::text::fingerprint::compute_segment_fingerprint(&crate::text::fingerprint::SegmentFingerprintInput {
                text: &text2,
                voice: &voice,
                model: "gemini-3.1-flash-tts-preview",
                speaking_rate: 1.0,
                pitch_shift: 0.0,
                volume_gain_db: 0.0,
                sample_rate_hz: 24000,
            });

            tx.execute(
                "UPDATE segments
                 SET text = ?1, prompt = ?2, fingerprint = ?3, status = 'pending', audio_path = NULL, duration_ms = 0, state_revision = state_revision + 1, updated_at = ?4
                 WHERE id = ?5",
                params![text1, prompt1, fp1, now, segment_id],
            )?;

            tx.execute(
                "UPDATE segments SET position = position + 1 WHERE project_id = ?1 AND position > ?2",
                params![project_id, seg.position as i64],
            )?;

            let new_seg_id = uuid::Uuid::new_v4().to_string();
            let new_pos = seg.position + 1;

            tx.execute(
                "INSERT INTO segments (
                    id, project_id, position, text, prompt, status, attempts, audio_path, duration_ms,
                    error_code, error_message, created_at, updated_at, fingerprint, output_fingerprint,
                    attempt_count, next_retry_at, queued_at, started_at, finished_at, lease_owner,
                    lease_expires_at, last_error_code, last_error_message, cancel_requested, state_revision, output_size
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, 'pending', 0, NULL, 0, NULL, NULL, ?6, ?6, ?7, NULL,
                    0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 1, 0
                )",
                params![
                    new_seg_id,
                    project_id,
                    new_pos as i64,
                    text2,
                    prompt2,
                    now,
                    fp2
                ],
            )?;

            tx.execute(
                "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
                params![now, project_id],
            )?;

            tx.commit()?;

            // Cleanup orphaned WAV file from the original segment being split
            if let Some(ref path_str) = seg.audio_path {
                let path = std::path::Path::new(path_str);
                if path.exists() {
                    match std::fs::remove_file(path) {
                        Ok(_) => info!("Cleaned up orphaned audio on split: {}", path_str),
                        Err(e) => warn!("Failed to cleanup audio on split {}: {}", path_str, e),
                    }
                }
            }

            Ok(())
        })
    }

    pub fn merge_segment_with_previous(
        db: &DatabaseManager,
        project_id: &str,
        segment_id: &str,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();

        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;

            // Get current segment
            let (current_pos, current_text): (i64, String) = tx.query_row(
                "SELECT position, text FROM segments WHERE id = ?1 AND project_id = ?2",
                rusqlite::params![segment_id, project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            if current_pos <= 1 {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Cannot merge: this is the first segment".to_string(),
                ));
            }

            let prev_pos = current_pos - 1;

            // Get previous segment
            let (prev_id, prev_text): (String, String) = tx.query_row(
                "SELECT id, text FROM segments WHERE project_id = ?1 AND position = ?2",
                rusqlite::params![project_id, prev_pos],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            // Merge text
            let merged_text = format!("{}\n{}", prev_text.trim(), current_text.trim());

            // Get project voice/preset for prompt rebuild
            let (voice, preset) = tx.query_row(
                "SELECT voice, preset FROM projects WHERE id = ?1",
                rusqlite::params![project_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            ).unwrap_or_else(|_| ("Kore".to_string(), "Tự nhiên".to_string()));

            let prompt_opts = crate::text::prompt_builder::PromptStyleOptions {
                style_preset: preset,
                pacing: "Bình thường".to_string(),
                pronunciation_notes: None,
            };
            let merged_prompt = crate::text::prompt_builder::build_tts_prompt(&merged_text, &prompt_opts);
            let merged_fp = crate::text::fingerprint::compute_segment_fingerprint(
                &crate::text::fingerprint::SegmentFingerprintInput {
                    text: &merged_text,
                    voice: &voice,
                    model: "gemini-3.1-flash-tts-preview",
                    speaking_rate: 1.0,
                    pitch_shift: 0.0,
                    volume_gain_db: 0.0,
                    sample_rate_hz: 24000,
                },
            );

            // Query audio paths of BOTH segments before modifying (previous will be overwritten, current deleted)
            let prev_audio_path: Option<String> = tx.query_row(
                "SELECT audio_path FROM segments WHERE id = ?1",
                rusqlite::params![prev_id],
                |row| row.get(0),
            ).optional()?.flatten();

            let current_audio_path: Option<String> = tx.query_row(
                "SELECT audio_path FROM segments WHERE id = ?1",
                rusqlite::params![segment_id],
                |row| row.get(0),
            ).optional()?.flatten();

            // Update previous segment with merged content
            tx.execute(
                "UPDATE segments SET text = ?1, prompt = ?2, fingerprint = ?3, status = 'pending', audio_path = NULL, duration_ms = 0, state_revision = state_revision + 1, updated_at = ?4 WHERE id = ?5",
                rusqlite::params![merged_text, merged_prompt, merged_fp, now, prev_id],
            )?;

            // Delete current segment
            tx.execute(
                "DELETE FROM segments WHERE id = ?1 AND project_id = ?2",
                rusqlite::params![segment_id, project_id],
            )?;

            // Reposition subsequent segments
            tx.execute(
                "UPDATE segments SET position = position - 1 WHERE project_id = ?1 AND position > ?2",
                rusqlite::params![project_id, current_pos],
            )?;

            tx.commit()?;

            // Cleanup orphaned WAV files from both merged segments
            for audio_path in [&prev_audio_path, &current_audio_path].iter().copied().flatten() {
                let path = std::path::Path::new(audio_path);
                if path.exists() {
                    match std::fs::remove_file(path) {
                        Ok(_) => info!("Cleaned up orphaned audio on merge: {}", audio_path),
                        Err(e) => warn!("Failed to cleanup audio on merge {}: {}", audio_path, e),
                    }
                }
            }

            Ok(())
        })
    }

    pub fn update_source_text(
        db: &DatabaseManager,
        project_id: &str,
        source_text: &str,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        db.with_conn(|conn| {
            conn.execute(
                "UPDATE projects SET source_text = ?1, updated_at = ?2 WHERE id = ?3",
                params![source_text, now, project_id],
            )?;
            Ok(())
        })
    }

    pub fn delete_segments_for_project(
        db: &DatabaseManager,
        project_id: &str,
    ) -> Result<(), String> {
        db.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let paths: Vec<String> = {
                let mut stmt = tx.prepare(
                    "SELECT audio_path FROM segments WHERE project_id = ?1 AND audio_path IS NOT NULL",
                )?;
                let res: Vec<String> = stmt
                    .query_map(params![project_id], |row| row.get(0))?
                    .filter_map(|r| r.ok())
                    .collect();
                res
            };
            tx.execute(
                "DELETE FROM segments WHERE project_id = ?1",
                params![project_id],
            )?;
            tx.commit()?;

            for path in paths {
                let _ = std::fs::remove_file(&path);
            }
            Ok(())
        })
    }

    pub fn update_segment_review_status(
        db: &DatabaseManager,
        segment_id: &str,
        review_status: &str,
        reviewed_output_fingerprint: Option<&str>,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        db.with_conn(|conn| {
            conn.execute(
                "UPDATE segments SET review_status = ?1, reviewed_output_fingerprint = ?2, updated_at = ?3 WHERE id = ?4",
                params![review_status, reviewed_output_fingerprint, now, segment_id],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_and_segments_crud() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = Utc::now().to_rfc3339();

        let proj = ProjectRecord {
            id: "p1".to_string(),
            name: "Test Project".to_string(),
            source_text: "Text".to_string(),
            model: "gemini-3.1-flash-tts-preview".to_string(),
            voice: "Kore".to_string(),
            preset: "Tự nhiên".to_string(),
            pacing: "Bình thường".to_string(),
            pronunciation_notes: None,
            output_directory: "C:/tmp".to_string(),
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        ProjectRepository::create_project(&db, &proj).unwrap();
        let projects = ProjectRepository::list_projects(&db).unwrap();
        assert_eq!(projects.len(), 1);

        let seg = SegmentRecord {
            id: "s1".to_string(),
            project_id: "p1".to_string(),
            position: 1,
            text: "Xin chào".to_string(),
            prompt: "Prompt".to_string(),
            status: SegmentStatus::Pending,
            attempts: 0,
            audio_path: None,
            duration_ms: 0,
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            fingerprint: Some("fp123".to_string()),
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
        };

        ProjectRepository::insert_segments(&db, &[seg]).unwrap();
        let segs = ProjectRepository::get_segments_for_project(&db, "p1").unwrap();
        assert_eq!(segs.len(), 1);

        ProjectRepository::mark_project_segments_queued(&db, "p1").unwrap();
        let claimed = ProjectRepository::claim_next_task(&db, "p1", "worker_1", 30).unwrap();
        assert!(claimed.is_some());
        let claimed_seg = claimed.unwrap();
        assert_eq!(claimed_seg.id, "s1");
        assert_eq!(claimed_seg.status, SegmentStatus::Processing);

        let committed = ProjectRepository::commit_task_result(
            &db,
            "s1",
            "worker_1",
            Some("fp123"),
            SegmentStatus::Success,
            Some("audio.wav"),
            4800,
            200,
            None,
            None,
            None,
        )
        .unwrap();
        assert!(committed);

        let final_segs = ProjectRepository::get_segments_for_project(&db, "p1").unwrap();
        assert_eq!(final_segs[0].status, SegmentStatus::Success);
        assert_eq!(final_segs[0].output_fingerprint.as_deref(), Some("fp123"));
    }

    #[test]
    fn test_get_project_and_updates() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = Utc::now().to_rfc3339();

        let proj = ProjectRecord {
            id: "p_test".to_string(),
            name: "Test Update Project".to_string(),
            source_text: "Original text".to_string(),
            model: "gemini-3.1-flash-tts-preview".to_string(),
            voice: "Kore".to_string(),
            preset: "Tự nhiên".to_string(),
            pacing: "Bình thường".to_string(),
            pronunciation_notes: None,
            output_directory: "C:/tmp".to_string(),
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        ProjectRepository::create_project(&db, &proj).unwrap();

        let fetched = ProjectRepository::get_project(&db, "p_test").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().voice, "Kore");

        ProjectRepository::update_project_voice(&db, "p_test", "Puck").unwrap();
        let updated_p = ProjectRepository::get_project(&db, "p_test")
            .unwrap()
            .unwrap();
        assert_eq!(updated_p.voice, "Puck");

        let seg = SegmentRecord {
            id: "s_test_1".to_string(),
            project_id: "p_test".to_string(),
            position: 1,
            text: "Hello World".to_string(),
            prompt: "Prompt 1".to_string(),
            status: SegmentStatus::Success,
            attempts: 1,
            audio_path: Some("path.wav".to_string()),
            duration_ms: 1000,
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            fingerprint: Some("fp1".to_string()),
            output_fingerprint: Some("ofp1".to_string()),
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
            output_size: 100,
            voice: None,
            synthesis_status: Some(SynthesisStatus::Success),
            review_status: Some(ReviewStatus::Unreviewed),
            reviewed_output_fingerprint: None,
        };

        ProjectRepository::insert_segments(&db, &[seg]).unwrap();

        ProjectRepository::update_segment_text(
            &db,
            "p_test",
            "s_test_1",
            "Updated Text",
            "New Prompt",
            "fp_new",
        )
        .unwrap();
        let segs = ProjectRepository::get_segments_for_project(&db, "p_test").unwrap();
        assert_eq!(segs[0].text, "Updated Text");
        assert_eq!(segs[0].status, SegmentStatus::Pending);
        assert!(segs[0].audio_path.is_none());

        // Test split segment
        ProjectRepository::split_segment(&db, "p_test", "s_test_1", 7).unwrap();
        let split_segs = ProjectRepository::get_segments_for_project(&db, "p_test").unwrap();
        assert_eq!(split_segs.len(), 2);
        assert_eq!(split_segs[0].position, 1);
        assert_eq!(split_segs[0].text, "Updated");
        assert_eq!(split_segs[1].position, 2);
        assert_eq!(split_segs[1].text, "Text");
    }

    #[test]
    fn test_get_project_by_id_and_segment_repo_helpers() {
        use crate::storage::segment_repo::SegmentRepository;

        let db = DatabaseManager::in_memory().unwrap();
        let now = Utc::now().to_rfc3339();

        let proj = ProjectRecord {
            id: "p_direct".to_string(),
            name: "Direct Query Project".to_string(),
            source_text: "Text payload".to_string(),
            model: "gemini-3.1-flash-tts-preview".to_string(),
            voice: "Kore".to_string(),
            preset: "Tự nhiên".to_string(),
            pacing: "Bình thường".to_string(),
            pronunciation_notes: None,
            output_directory: "C:/tmp".to_string(),
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        ProjectRepository::create_project(&db, &proj).unwrap();

        // Verify get_project_by_id
        let fetched = ProjectRepository::get_project_by_id(&db, "p_direct").unwrap();
        assert!(fetched.is_some());
        let record = fetched.unwrap();
        assert_eq!(record.id, "p_direct");
        assert_eq!(record.name, "Direct Query Project");

        // Verify non-existent project returns None
        let not_found = ProjectRepository::get_project_by_id(&db, "non_existent").unwrap();
        assert!(not_found.is_none());

        // Verify ProjectRepository::update_voice
        ProjectRepository::update_voice(&db, "p_direct", "Puck").unwrap();
        let updated_proj = ProjectRepository::get_project_by_id(&db, "p_direct")
            .unwrap()
            .unwrap();
        assert_eq!(updated_proj.voice, "Puck");

        // Verify update_voice on non-existent project returns Err
        assert!(ProjectRepository::update_voice(&db, "non_existent_proj", "Puck").is_err());

        // Create a segment for SegmentRepository tests
        let seg = SegmentRecord {
            id: "s_seg_repo".to_string(),
            project_id: "p_direct".to_string(),
            position: 1,
            text: "Initial segment text".to_string(),
            prompt: "Prompt".to_string(),
            status: SegmentStatus::Pending,
            attempts: 0,
            audio_path: None,
            duration_ms: 0,
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            fingerprint: Some("fp_init".to_string()),
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
        };
        ProjectRepository::insert_segments(&db, &[seg]).unwrap();

        // Test SegmentRepository::update_text
        SegmentRepository::update_text(
            &db,
            "p_direct",
            "s_seg_repo",
            "Modified text",
            "Prompt new",
            "fp_mod",
        )
        .unwrap();
        let segs = ProjectRepository::get_segments_for_project(&db, "p_direct").unwrap();
        assert_eq!(segs[0].text, "Modified text");

        // Test SegmentRepository::split_segment
        SegmentRepository::split_segment(&db, "p_direct", "s_seg_repo", 8).unwrap();
        let split_segs = ProjectRepository::get_segments_for_project(&db, "p_direct").unwrap();
        assert_eq!(split_segs.len(), 2);
        assert_eq!(split_segs[0].text, "Modified");
        assert_eq!(split_segs[1].text, "text");
    }

    #[test]
    fn test_delete_project_removes_audio_files() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = Utc::now().to_rfc3339();

        let proj = ProjectRecord {
            id: "p_audio_cleanup".to_string(),
            name: "Audio Cleanup Test Project".to_string(),
            source_text: "Text".to_string(),
            model: "gemini-3.1-flash-tts-preview".to_string(),
            voice: "Kore".to_string(),
            preset: "Tự nhiên".to_string(),
            pacing: "Bình thường".to_string(),
            pronunciation_notes: None,
            output_directory: "C:/tmp".to_string(),
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        ProjectRepository::create_project(&db, &proj).unwrap();

        let temp_audio_dir = std::env::temp_dir();
        let temp_file_path = temp_audio_dir.join("test_cleanup_proj_audio.wav");
        std::fs::write(&temp_file_path, b"dummy wav content").unwrap();
        assert!(temp_file_path.exists());

        let seg = SegmentRecord {
            id: "s_audio_cleanup".to_string(),
            project_id: "p_audio_cleanup".to_string(),
            position: 1,
            text: "Audio test segment".to_string(),
            prompt: "Prompt".to_string(),
            status: SegmentStatus::Success,
            attempts: 1,
            audio_path: Some(temp_file_path.to_string_lossy().to_string()),
            duration_ms: 1000,
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            fingerprint: Some("fp_clean".to_string()),
            output_fingerprint: Some("ofp_clean".to_string()),
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
            output_size: 100,
            voice: None,
            synthesis_status: Some(SynthesisStatus::Success),
            review_status: Some(ReviewStatus::Unreviewed),
            reviewed_output_fingerprint: None,
        };
        ProjectRepository::insert_segments(&db, &[seg]).unwrap();

        ProjectRepository::delete_project(&db, "p_audio_cleanup").unwrap();

        assert!(ProjectRepository::get_project(&db, "p_audio_cleanup")
            .unwrap()
            .is_none());
        assert!(!temp_file_path.exists());
    }

    #[test]
    fn test_delete_segment_removes_audio_file() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = Utc::now().to_rfc3339();

        let proj = ProjectRecord {
            id: "p_seg_cleanup".to_string(),
            name: "Segment Cleanup Test Project".to_string(),
            source_text: "Text".to_string(),
            model: "gemini-3.1-flash-tts-preview".to_string(),
            voice: "Kore".to_string(),
            preset: "Tự nhiên".to_string(),
            pacing: "Bình thường".to_string(),
            pronunciation_notes: None,
            output_directory: "C:/tmp".to_string(),
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        ProjectRepository::create_project(&db, &proj).unwrap();

        let temp_audio_dir = std::env::temp_dir();
        let temp_file_path = temp_audio_dir.join("test_cleanup_seg_audio.wav");
        std::fs::write(&temp_file_path, b"dummy wav content").unwrap();
        assert!(temp_file_path.exists());

        let seg = SegmentRecord {
            id: "s_seg_cleanup".to_string(),
            project_id: "p_seg_cleanup".to_string(),
            position: 1,
            text: "Segment text".to_string(),
            prompt: "Prompt".to_string(),
            status: SegmentStatus::Success,
            attempts: 1,
            audio_path: Some(temp_file_path.to_string_lossy().to_string()),
            duration_ms: 1000,
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            fingerprint: Some("fp_seg_clean".to_string()),
            output_fingerprint: Some("ofp_seg_clean".to_string()),
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
            output_size: 100,
            voice: None,
            synthesis_status: Some(SynthesisStatus::Success),
            review_status: Some(ReviewStatus::Unreviewed),
            reviewed_output_fingerprint: None,
        };
        ProjectRepository::insert_segments(&db, &[seg]).unwrap();

        ProjectRepository::delete_segment(&db, "p_seg_cleanup", "s_seg_cleanup").unwrap();

        let segs = ProjectRepository::get_segments_for_project(&db, "p_seg_cleanup").unwrap();
        assert_eq!(segs.len(), 0);
        assert!(!temp_file_path.exists());
    }

    #[test]
    fn test_chunked_batch_insert_large_segments() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = Utc::now().to_rfc3339();

        let proj = ProjectRecord {
            id: "p_batch_test".to_string(),
            name: "Batch Insert Test Project".to_string(),
            source_text: "Long book text".to_string(),
            model: "gemini-3.1-flash-tts-preview".to_string(),
            voice: "Kore".to_string(),
            preset: "Tự nhiên".to_string(),
            pacing: "Bình thường".to_string(),
            pronunciation_notes: None,
            output_directory: "C:/tmp".to_string(),
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        ProjectRepository::create_project(&db, &proj).unwrap();

        let mut segments = Vec::new();
        for i in 1..=1200 {
            segments.push(SegmentRecord {
                id: format!("seg_batch_{}", i),
                project_id: "p_batch_test".to_string(),
                position: i,
                text: format!("Đoạn văn bản thứ {} trong dự án lớn.", i),
                prompt: "Đọc tự nhiên".to_string(),
                status: SegmentStatus::Pending,
                attempts: 0,
                audio_path: None,
                duration_ms: 500,
                error_code: None,
                error_message: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                fingerprint: Some(format!("fp_{}", i)),
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
            });
        }

        ProjectRepository::insert_segments(&db, &segments).unwrap();

        let fetched_segments =
            ProjectRepository::get_segments_for_project(&db, "p_batch_test").unwrap();
        assert_eq!(fetched_segments.len(), 1200);
        assert_eq!(fetched_segments[0].position, 1);
        assert_eq!(fetched_segments[1199].position, 1200);
    }
}
