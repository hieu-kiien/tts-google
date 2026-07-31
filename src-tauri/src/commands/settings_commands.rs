use std::sync::atomic::Ordering;
use tauri::State;
use serde::{Deserialize, Serialize};
use crate::state::app_state::AppState;
use crate::storage::db::QuotaMetrics;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub concurrency: u32,
}

#[tauri::command]
pub async fn get_app_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let concurrency = state.concurrency.load(Ordering::Relaxed);
    Ok(AppSettings { concurrency })
}

#[tauri::command]
pub async fn update_app_settings(
    concurrency: u32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let valid_concurrency = concurrency.clamp(1, 5);
    state.concurrency.store(valid_concurrency, Ordering::Relaxed);

    if let Some(ref db) = state.db {
        let _ = db.set_setting("concurrency", &valid_concurrency.to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn get_quota_metrics(
    state: State<'_, AppState>,
) -> Result<QuotaMetrics, String> {
    if let Some(ref db) = state.db {
        let mut metrics = db.get_quota_metrics().unwrap_or_default();
        // Combine in-memory counters for live updates
        let mem_reqs = state.total_requests.load(Ordering::Relaxed);
        let mem_chars = state.total_chars.load(Ordering::Relaxed);
        let mem_rate_limits = state.rate_limit_hits.load(Ordering::Relaxed);

        if mem_reqs > 0 {
            metrics.today_requests += mem_reqs;
            metrics.today_chars += mem_chars;
            metrics.today_rate_limits += mem_rate_limits;
            metrics.total_requests += mem_reqs;
            metrics.total_chars += mem_chars;
        }

        Ok(metrics)
    } else {
        Ok(QuotaMetrics {
            today_requests: state.total_requests.load(Ordering::Relaxed),
            today_chars: state.total_chars.load(Ordering::Relaxed),
            today_rate_limits: state.rate_limit_hits.load(Ordering::Relaxed),
            avg_latency_ms: 0,
            total_requests: state.total_requests.load(Ordering::Relaxed),
            total_chars: state.total_chars.load(Ordering::Relaxed),
        })
    }
}
