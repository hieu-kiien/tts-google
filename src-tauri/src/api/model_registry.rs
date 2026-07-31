use serde::{Deserialize, Serialize};

pub const DEFAULT_FREE_TIER_MODEL: &str = "gemini-3.1-flash-tts-preview";
pub const FALLBACK_FREE_TIER_MODEL: &str = "gemini-2.5-flash-preview-tts";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub allowed_models: Vec<String>,
    pub default_model: String,
    pub fallback_model: String,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self {
            allowed_models: vec![
                DEFAULT_FREE_TIER_MODEL.to_string(),
                FALLBACK_FREE_TIER_MODEL.to_string(),
            ],
            default_model: DEFAULT_FREE_TIER_MODEL.to_string(),
            fallback_model: FALLBACK_FREE_TIER_MODEL.to_string(),
        }
    }
}

impl ModelRegistry {
    pub fn is_model_allowed(&self, model_id: &str) -> bool {
        self.allowed_models.iter().any(|m| m == model_id)
    }

    pub fn sanitize_model_choice(&self, requested_model: Option<&str>) -> String {
        match requested_model {
            Some(m) if self.is_model_allowed(m) => m.to_string(),
            _ => self.default_model.clone(),
        }
    }
}
