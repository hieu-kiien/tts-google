use crate::state::app_state::AppState;
use tauri::State;

#[tauri::command]
pub async fn ai_translate_text(
    text: String,
    target_lang: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("Văn bản rỗng, vui lòng nhập nội dung cần dịch".to_string());
    }

    let api_key = state.credentials.get_key().ok_or_else(|| {
        "Chưa cấu hình Gemini API Key. Vui lòng bấm 'Cấu Hình API Key' để dán key.".to_string()
    })?;

    let prompt = format!(
        "Bạn là một biên dịch viên chuyên nghiệp. Hãy dịch toàn bộ đoạn văn bản sau sang {}. Chỉ trả về duy nhất bản dịch kết quả, không thêm lời chào hay giải thích:\n\n{}",
        target_lang,
        text
    );

    let translated = state
        .gemini_client
        .generate_text(&api_key, &prompt)
        .await
        .map_err(|e| format!("Lỗi dịch thuật AI: {}", e))?;

    Ok(translated.trim().to_string())
}

#[tauri::command]
pub async fn ai_polish_text(text: String, state: State<'_, AppState>) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("Văn bản rỗng, vui lòng nhập nội dung cần tối ưu".to_string());
    }

    let api_key = state.credentials.get_key().ok_or_else(|| {
        "Chưa cấu hình Gemini API Key. Vui lòng bấm 'Cấu Hình API Key' để dán key.".to_string()
    })?;

    let prompt = format!(
        "Bạn là một biên tập viên kịch bản giọng đọc TTS chuyên nghiệp. Hãy chỉnh sửa văn bản sau cho chuẩn tiếng Việt: sửa lỗi chính tả, thêm đầy đủ dấu chấm câu, chuẩn hóa từ ngữ giúp AI đọc giọng nói tự nhiên và mượt mà nhất. Chỉ trả về duy nhất văn bản đã tối ưu:\n\n{}",
        text
    );

    let polished = state
        .gemini_client
        .generate_text(&api_key, &prompt)
        .await
        .map_err(|e| format!("Lỗi tối ưu văn bản AI: {}", e))?;

    Ok(polished.trim().to_string())
}
