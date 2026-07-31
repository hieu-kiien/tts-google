use tauri::State;
use serde::{Serialize, Deserialize};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use crate::state::app_state::AppState;
use crate::audio::pcm_wav::pcm_to_wav_bytes;
use crate::api::interactions_client::DEFAULT_MODEL;
use crate::security::input_validation::validate_api_key;
use crate::models::registry::validate_tts_model;

#[derive(Serialize, Deserialize)]
pub struct KeyStatus {
    pub configured: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AudioPreviewResult {
    pub data_url: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub byte_size: usize,
}

#[tauri::command]
pub fn save_api_key(
    key: String,
    remember: bool,
    state: State<'_, AppState>,
) -> Result<KeyStatus, String> {
    validate_api_key(&key)?;
    state.credentials.set_key(&key, remember)?;
    Ok(KeyStatus {
        configured: state.credentials.is_configured(),
    })
}

#[tauri::command]
pub fn get_api_key_status(state: State<'_, AppState>) -> KeyStatus {
    KeyStatus {
        configured: state.credentials.is_configured(),
    }
}

#[tauri::command]
pub fn delete_api_key(state: State<'_, AppState>) -> Result<KeyStatus, String> {
    state.credentials.delete_key()?;
    Ok(KeyStatus { configured: false })
}

// ─── Multi-key API commands ───

#[derive(Serialize, Deserialize)]
pub struct MultiKeyStatus {
    pub count: usize,
    pub keys_masked: Vec<String>,
    pub configured: bool,
}

#[tauri::command]
pub fn save_api_keys(
    keys: Vec<String>,
    remember: bool,
    state: State<'_, AppState>,
) -> Result<MultiKeyStatus, String> {
    for key in &keys {
        validate_api_key(key)?;
    }
    let count = state.credentials.set_keys(keys, remember)?;
    Ok(MultiKeyStatus {
        count,
        keys_masked: state.credentials.get_keys_masked(),
        configured: state.credentials.is_configured(),
    })
}

#[tauri::command]
pub fn get_api_keys_info(state: State<'_, AppState>) -> MultiKeyStatus {
    MultiKeyStatus {
        count: state.credentials.key_count(),
        keys_masked: state.credentials.get_keys_masked(),
        configured: state.credentials.is_configured(),
    }
}

#[tauri::command]
pub fn remove_api_key_at(
    index: usize,
    state: State<'_, AppState>,
) -> Result<MultiKeyStatus, String> {
    state.credentials.remove_key_at(index)?;
    Ok(MultiKeyStatus {
        count: state.credentials.key_count(),
        keys_masked: state.credentials.get_keys_masked(),
        configured: state.credentials.is_configured(),
    })
}

#[tauri::command]
pub async fn test_api_connection(
    test_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let key = match test_key {
        Some(k) if !k.trim().is_empty() => {
            validate_api_key(&k)?;
            k
        }
        _ => state
            .credentials
            .get_key()
            .ok_or_else(|| "Vui lòng nhập hoặc lưu Gemini API Key trước khi test".to_string())?,
    };

    let url = "https://generativelanguage.googleapis.com/v1beta/models";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Lỗi tạo HTTP Client: {}", e))?;

    let response = client
        .get(url)
        .header("x-goog-api-key", key.trim())
        .send()
        .await
        .map_err(|e| format!("Không thể kết nối máy chủ Google API (Kiểm tra kết nối mạng): {}", e))?;

    let status = response.status();
    if status.is_success() {
        Ok("✅ API Key hợp lệ! Kết nối Google Gemini API siêu tốc thành công!".to_string())
    } else if status.as_u16() == 400 || status.as_u16() == 401 || status.as_u16() == 403 {
        Err("❌ API Key không hợp lệ hoặc đã bị vô hiệu hóa (HTTP 400/401/403)".to_string())
    } else if status.as_u16() == 429 {
        Err("⚠️ Quá giới hạn lượt gọi API (Rate Limited - 429). Vui lòng thử lại sau vài giây.".to_string())
    } else {
        let err_body = response.text().await.unwrap_or_default();
        let safe_err = err_body.replace(key.trim(), "***REDACTED***");
        Err(format!("Lỗi Google API (HTTP {}): {}", status.as_u16(), safe_err))
    }
}

#[tauri::command]
pub async fn synthesize_preview_audio(
    text: String,
    voice: String,
    model: Option<String>,
    speed: Option<f32>,
    pitch: Option<f32>,
    state: State<'_, AppState>,
) -> Result<AudioPreviewResult, String> {
    let key = state
        .credentials
        .get_key()
        .ok_or_else(|| "Chưa cấu hình Gemini API Key. Vui lòng bấm 'Cấu Hình API Key' để dán key.".to_string())?;

    let selected_model = match model {
        Some(m) if !m.trim().is_empty() => {
            validate_tts_model(&m)?;
            m
        }
        _ => DEFAULT_MODEL.to_string(),
    };

    let formatted_text = match (speed, pitch) {
        (Some(s), Some(p)) if (s - 1.0).abs() > 0.05 || (p - 1.0).abs() > 0.05 => {
            format!("[Direction: Speed {:.2}x, Pitch {:.2}] {}", s, p, text)
        }
        (Some(s), _) if (s - 1.0).abs() > 0.05 => {
            format!("[Direction: Speed {:.2}x] {}", s, text)
        }
        _ => text,
    };

    let pcm_bytes = match state
        .gemini_client
        .synthesize_speech(&key, &selected_model, &formatted_text, &voice)
        .await
    {
        Ok(bytes) => bytes,
        Err(_e) => {
            state
                .gemini_client
                .synthesize_speech(&key, DEFAULT_MODEL, &formatted_text, &voice)
                .await
                .map_err(|err| format!("Lỗi tổng hợp audio từ Gemini API: {}", err))?
        }
    };

    let wav_bytes = pcm_to_wav_bytes(&pcm_bytes)?;
    let base64_wav = BASE64_STANDARD.encode(&wav_bytes);
    let data_url = format!("data:audio/wav;base64,{}", base64_wav);

    let duration_ms = (pcm_bytes.len() as u64 * 1000) / 48000;

    Ok(AudioPreviewResult {
        data_url,
        duration_ms,
        sample_rate: 24000,
        byte_size: wav_bytes.len(),
    })
}
