use crate::api::interactions_client::{ApiError, GeminiClient, DEFAULT_MODEL, FALLBACK_MODEL};
use crate::models::registry::{list_available_models, TtsModelDescriptor};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisConfig {
    pub model: String,
    pub voice: String,
    pub speaking_rate: f32,
    pub pitch_shift: f32,
    pub volume_gain_db: f32,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            voice: "Kore".to_string(),
            speaking_rate: 1.0,
            pitch_shift: 0.0,
            volume_gain_db: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentials {
    pub api_key: String,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SynthesisOutput {
    pub audio_bytes: Vec<u8>,
    pub mime_type: String,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub provider_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderError {
    pub code: String,
    pub message: String,
    pub is_rate_limit: bool,
    pub retry_after_secs: Option<u64>,
}

impl From<ApiError> for ProviderError {
    fn from(err: ApiError) -> Self {
        let is_rate_limit = match err {
            ApiError::RateLimited(_) | ApiError::RateLimitedDaily => true,
            _ => false,
        };
        let retry_after_secs = match err {
            ApiError::RateLimited(retry_after) => retry_after,
            _ => None,
        };
        Self {
            code: format!("{:?}", err),
            message: err.to_string(),
            is_rate_limit,
            retry_after_secs,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub character_count: usize,
    pub estimated_tokens: usize,
    pub is_free_tier: bool,
    pub estimated_cost_usd: f64,
}

#[async_trait]
pub trait TtsProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn models(&self) -> Vec<TtsModelDescriptor>;
    fn list_voices(&self) -> Vec<VoiceInfo>;
    async fn synthesize(
        &self,
        credentials: &ProviderCredentials,
        text: &str,
        config: &SynthesisConfig,
    ) -> Result<SynthesisOutput, ProviderError>;
    fn estimate_cost(&self, text: &str) -> CostEstimate;
}

pub struct GeminiProvider {
    client: GeminiClient,
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self {
            client: GeminiClient::new(),
        }
    }
}

#[async_trait]
impl TtsProvider for GeminiProvider {
    fn id(&self) -> &'static str {
        "google_gemini"
    }

    fn name(&self) -> &'static str {
        "Google Gemini TTS"
    }

    fn models(&self) -> Vec<TtsModelDescriptor> {
        list_available_models()
    }

    fn list_voices(&self) -> Vec<VoiceInfo> {
        vec![
            VoiceInfo {
                id: "Kore".to_string(),
                name: "Kore".to_string(),
                gender: "Female".to_string(),
                description: "Trầm ấm, tự nhiên".to_string(),
            },
            VoiceInfo {
                id: "Aoede".to_string(),
                name: "Aoede".to_string(),
                gender: "Female".to_string(),
                description: "Truyền cảm, sâu lắng".to_string(),
            },
            VoiceInfo {
                id: "Zephyr".to_string(),
                name: "Zephyr".to_string(),
                gender: "Female".to_string(),
                description: "Nhẹ nhàng, trong trẻo".to_string(),
            },
            VoiceInfo {
                id: "Calliope".to_string(),
                name: "Calliope".to_string(),
                gender: "Female".to_string(),
                description: "Kể chuyện, ngọt ngào".to_string(),
            },
            VoiceInfo {
                id: "Leda".to_string(),
                name: "Leda".to_string(),
                gender: "Female".to_string(),
                description: "Thanh lịch, điềm tĩnh".to_string(),
            },
            VoiceInfo {
                id: "Puck".to_string(),
                name: "Puck".to_string(),
                gender: "Male".to_string(),
                description: "Năng động, trẻ trung".to_string(),
            },
            VoiceInfo {
                id: "Charon".to_string(),
                name: "Charon".to_string(),
                gender: "Male".to_string(),
                description: "Trang trọng, đọc tin".to_string(),
            },
            VoiceInfo {
                id: "Fenrir".to_string(),
                name: "Fenrir".to_string(),
                gender: "Male".to_string(),
                description: "Mạnh mẽ, cuốn hút".to_string(),
            },
            VoiceInfo {
                id: "Orpheus".to_string(),
                name: "Orpheus".to_string(),
                gender: "Male".to_string(),
                description: "Ấm áp, diễn cảm".to_string(),
            },
            VoiceInfo {
                id: "Pegasus".to_string(),
                name: "Pegasus".to_string(),
                gender: "Male".to_string(),
                description: "Quyền lực, rõ ràng".to_string(),
            },
            VoiceInfo {
                id: "Mimas".to_string(),
                name: "Mimas".to_string(),
                gender: "Male".to_string(),
                description: "Trầm thấp, thần thái".to_string(),
            },
        ]
    }

    async fn synthesize(
        &self,
        credentials: &ProviderCredentials,
        text: &str,
        config: &SynthesisConfig,
    ) -> Result<SynthesisOutput, ProviderError> {
        let selected_model = if config.model.trim().is_empty() {
            DEFAULT_MODEL
        } else {
            &config.model
        };

        let safe_model = if selected_model == DEFAULT_MODEL || selected_model == FALLBACK_MODEL {
            selected_model
        } else {
            DEFAULT_MODEL
        };

        let pcm_bytes = self
            .client
            .synthesize_speech(&credentials.api_key, safe_model, text, &config.voice)
            .await?;

        Ok(SynthesisOutput {
            audio_bytes: pcm_bytes,
            mime_type: "audio/pcm;rate=24000".to_string(),
            sample_rate_hz: 24000,
            channels: 1,
            provider_request_id: None,
        })
    }

    fn estimate_cost(&self, text: &str) -> CostEstimate {
        let char_count = text.chars().count();
        let estimated_tokens = (char_count as f64 * 0.25).ceil() as usize;
        CostEstimate {
            character_count: char_count,
            estimated_tokens,
            is_free_tier: true,
            estimated_cost_usd: 0.0,
        }
    }
}

pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn TtsProvider>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        let mut reg = Self {
            providers: HashMap::new(),
        };
        reg.register(Box::new(GeminiProvider::new()));
        reg
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Box<dyn TtsProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    pub fn get(&self, provider_id: &str) -> Option<&dyn TtsProvider> {
        self.providers.get(provider_id).map(|b| b.as_ref())
    }

    pub fn default_provider(&self) -> &dyn TtsProvider {
        self.get("google_gemini")
            .expect("Default provider google_gemini missing")
    }
}
