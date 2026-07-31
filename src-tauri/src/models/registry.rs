pub const MODEL_GEMINI_31_FLASH_TTS: &str = "gemini-3.1-flash-tts-preview";
pub const MODEL_GEMINI_25_FLASH_TTS: &str = "gemini-2.5-flash-preview-tts";

pub fn validate_tts_model(model: &str) -> Result<(), String> {
    match model.trim() {
        MODEL_GEMINI_31_FLASH_TTS | MODEL_GEMINI_25_FLASH_TTS => Ok(()),
        _ => Err(format!(
            "TTS Model '{}' không được hỗ trợ. Các model hỗ trợ: ['{}', '{}']",
            model, MODEL_GEMINI_31_FLASH_TTS, MODEL_GEMINI_25_FLASH_TTS
        )),
    }
}
