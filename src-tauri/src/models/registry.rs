use serde::{Deserialize, Serialize};

pub const MODEL_GEMINI_31_FLASH_TTS: &str = "gemini-3.1-flash-tts-preview";
pub const MODEL_GEMINI_25_FLASH_TTS: &str = "gemini-2.5-flash-preview-tts";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelLifecycle {
    Preview,
    Stable,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub supports_single_speaker: bool,
    pub supports_multi_speaker: bool,
    pub max_input_tokens: Option<u32>,
    pub lifecycle: ModelLifecycle,
}

pub fn list_available_models() -> Vec<TtsModelDescriptor> {
    vec![
        TtsModelDescriptor {
            id: MODEL_GEMINI_31_FLASH_TTS.to_string(),
            display_name: "Gemini 3.1 Flash TTS Preview".to_string(),
            supports_single_speaker: true,
            supports_multi_speaker: true,
            max_input_tokens: Some(8192),
            lifecycle: ModelLifecycle::Preview,
        },
        TtsModelDescriptor {
            id: MODEL_GEMINI_25_FLASH_TTS.to_string(),
            display_name: "Gemini 2.5 Flash TTS Preview".to_string(),
            supports_single_speaker: true,
            supports_multi_speaker: false,
            max_input_tokens: Some(4096),
            lifecycle: ModelLifecycle::Preview,
        },
    ]
}

pub fn validate_tts_model(model: &str) -> Result<(), String> {
    match model.trim() {
        MODEL_GEMINI_31_FLASH_TTS | MODEL_GEMINI_25_FLASH_TTS => Ok(()),
        _ => Err(format!(
            "TTS Model '{}' không được hỗ trợ. Các model hỗ trợ: ['{}', '{}']",
            model, MODEL_GEMINI_31_FLASH_TTS, MODEL_GEMINI_25_FLASH_TTS
        )),
    }
}
