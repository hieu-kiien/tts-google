pub mod api;
pub mod audio;
pub mod commands;
pub mod error;
pub mod models;
pub mod queue;
pub mod security;
pub mod state;
pub mod storage;
pub mod text;

#[cfg(test)]
pub mod integration_tests;

use queue::worker::QueueService;
use state::app_state::AppState;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let app_state_base = Arc::new(AppState::new());
            let queue_service = Arc::new(QueueService::new(Arc::clone(&app_state_base), handle));

            let final_state = AppState {
                credentials: Arc::clone(&app_state_base.credentials),
                gemini_client: Arc::clone(&app_state_base.gemini_client),
                db: app_state_base.db.clone(),
                app_data_dir: app_state_base.app_data_dir.clone(),
                output_dir: app_state_base.output_dir.clone(),
                temp_dir: app_state_base.temp_dir.clone(),
                queue_service: Some(queue_service),
                concurrency: std::sync::atomic::AtomicU32::new(
                    app_state_base
                        .concurrency
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
                total_requests: std::sync::atomic::AtomicU64::new(0),
                total_chars: std::sync::atomic::AtomicU64::new(0),
                rate_limit_hits: std::sync::atomic::AtomicU64::new(0),
                total_latency_ms: std::sync::atomic::AtomicU64::new(0),
            };

            app.manage(final_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::api_commands::save_api_key,
            commands::api_commands::get_api_key_status,
            commands::api_commands::delete_api_key,
            commands::api_commands::save_api_keys,
            commands::api_commands::get_api_keys_info,
            commands::api_commands::remove_api_key_at,
            commands::api_commands::test_api_connection,
            commands::api_commands::synthesize_preview_audio,
            commands::project_commands::create_project,
            commands::project_commands::list_projects,
            commands::project_commands::delete_project,
            commands::project_commands::delete_projects_batch,
            commands::project_commands::delete_segment,
            commands::project_commands::delete_segments_batch,
            commands::project_commands::get_project_segments,
            commands::project_commands::normalize_vietnamese_text,
            commands::project_commands::update_segment_text,
            commands::project_commands::update_segment_voice,
            commands::project_commands::update_segment_review_status,
            commands::project_commands::update_project_voice,
            commands::project_commands::split_segment,
            commands::project_commands::merge_segments,
            commands::project_commands::chunk_text_preview,
            commands::project_commands::rechunk_project_segments,
            commands::project_commands::insert_segment_at,
            commands::project_commands::move_segment,
            commands::audio_commands::merge_project_audio,
            commands::audio_commands::export_project_srt,
            commands::audio_commands::export_project_vtt,
            commands::audio_commands::export_project_lrc,
            commands::audio_commands::export_project_m4b_manifest,
            commands::audio_commands::save_single_segment_audio,
            commands::audio_commands::read_audio_data_url,
            commands::audio_commands::write_binary_file,
            commands::dialog_commands::pick_output_folder,
            commands::dialog_commands::save_master_wav_dialog,
            commands::dialog_commands::save_srt_file_dialog,
            commands::dialog_commands::save_vtt_file_dialog,
            commands::dialog_commands::save_segment_audio_dialog,
            commands::dialog_commands::read_text_file_dialog,
            commands::translate_commands::ai_translate_text,
            commands::translate_commands::ai_polish_text,
            commands::queue_commands::enqueue_project,
            commands::queue_commands::pause_project,
            commands::queue_commands::resume_project,
            commands::queue_commands::cancel_project,
            commands::queue_commands::get_queue_snapshot,
            commands::queue_commands::check_export_readiness,
            commands::queue_commands::requeue_segment,
            commands::settings_commands::get_app_settings,
            commands::settings_commands::update_app_settings,
            commands::settings_commands::get_quota_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
