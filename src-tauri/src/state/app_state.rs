use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64};
use crate::security::keyring_store::CredentialStore;
use crate::api::interactions_client::GeminiClient;
use crate::storage::db::DatabaseManager;
use crate::queue::worker::QueueService;

pub struct AppState {
    pub credentials: Arc<CredentialStore>,
    pub gemini_client: Arc<GeminiClient>,
    pub db: Option<Arc<DatabaseManager>>,
    pub app_data_dir: PathBuf,
    pub output_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub queue_service: Option<Arc<QueueService>>,
    pub concurrency: AtomicU32,
    pub total_requests: AtomicU64,
    pub total_chars: AtomicU64,
    pub rate_limit_hits: AtomicU64,
    pub total_latency_ms: AtomicU64,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let app_dir = if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA")
                .map(|p| PathBuf::from(p).join("AutoTTSDesktop"))
                .unwrap_or_else(|_| std::env::temp_dir().join("auto_tts_desktop"))
        } else {
            dirs_or_fallback()
        };

        let output_dir = app_dir.join("output");
        let temp_dir = app_dir.join("temp");

        let _ = std::fs::create_dir_all(&app_dir);
        let _ = std::fs::create_dir_all(&output_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let db_path = app_dir.join("autotts.db");
        let db = DatabaseManager::new(&db_path).ok().map(Arc::new);

        // Load saved concurrency if present
        let saved_concurrency = if let Some(ref d) = db {
            d.get_setting("concurrency")
                .ok()
                .flatten()
                .and_then(|val| val.parse::<u32>().ok())
                .unwrap_or(1)
                .clamp(1, 5)
        } else {
            1
        };

        Self {
            credentials: Arc::new(CredentialStore::new()),
            gemini_client: Arc::new(GeminiClient::new()),
            db,
            app_data_dir: app_dir,
            output_dir,
            temp_dir,
            queue_service: None,
            concurrency: AtomicU32::new(saved_concurrency),
            total_requests: AtomicU64::new(0),
            total_chars: AtomicU64::new(0),
            rate_limit_hits: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
        }
    }

    pub fn get_allowed_roots(&self) -> Vec<&Path> {
        vec![
            self.app_data_dir.as_path(),
            self.output_dir.as_path(),
            self.temp_dir.as_path(),
        ]
    }
}

fn dirs_or_fallback() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(|p| PathBuf::from(p).join("auto_tts_desktop"))
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".local").join("share").join("auto_tts_desktop"))
                .unwrap_or_else(|_| std::env::temp_dir().join("auto_tts_desktop"))
        })
}
