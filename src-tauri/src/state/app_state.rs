use std::sync::Arc;
use crate::security::keyring_store::CredentialStore;
use crate::api::interactions_client::GeminiClient;
use crate::storage::db::DatabaseManager;
use crate::queue::worker::QueueService;

pub struct AppState {
    pub credentials: Arc<CredentialStore>,
    pub gemini_client: Arc<GeminiClient>,
    pub db: Option<Arc<DatabaseManager>>,
    pub output_dir: String,
    pub queue_service: Option<Arc<QueueService>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        // Use proper persistent data directory instead of temp
        let app_dir = if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA")
                .map(|p| std::path::PathBuf::from(p).join("AutoTTSDesktop"))
                .unwrap_or_else(|_| std::env::temp_dir().join("auto_tts_desktop"))
        } else {
            dirs_or_fallback()
        };
        let _ = std::fs::create_dir_all(&app_dir);
        let db_path = app_dir.join("autotts.db");
        let db = DatabaseManager::new(&db_path).ok().map(Arc::new);

        Self {
            credentials: Arc::new(CredentialStore::new()),
            gemini_client: Arc::new(GeminiClient::new()),
            db,
            output_dir: app_dir.to_str().unwrap_or("./").to_string(),
            queue_service: None,
        }
    }
}

fn dirs_or_fallback() -> std::path::PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(|p| std::path::PathBuf::from(p).join("auto_tts_desktop"))
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".local").join("share").join("auto_tts_desktop"))
                .unwrap_or_else(|_| std::env::temp_dir().join("auto_tts_desktop"))
        })
}
