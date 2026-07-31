use serde::{Deserialize, Serialize};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use tracing::{info, warn};

pub const DEFAULT_MODEL: &str = "gemini-3.1-flash-tts-preview";
pub const FALLBACK_MODEL: &str = "gemini-2.5-flash-preview-tts";

// Official Google Gemini API Endpoint Template
// POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
pub fn get_generate_content_endpoint(model: &str) -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrebuiltVoiceConfig {
    #[serde(rename = "voiceName")]
    pub voice_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceConfig {
    #[serde(rename = "prebuiltVoiceConfig")]
    pub prebuilt_voice_config: PrebuiltVoiceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechConfig {
    #[serde(rename = "voiceConfig")]
    pub voice_config: VoiceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationConfig {
    #[serde(rename = "responseModalities")]
    pub response_modalities: Vec<String>,
    #[serde(rename = "speechConfig", skip_serializing_if = "Option::is_none")]
    pub speech_config: Option<SpeechConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<InlineData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiGenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    pub generation_config: GenerationConfig,
}

impl GeminiGenerateContentRequest {
    pub fn new_tts_request(text: &str, voice: &str) -> Self {
        Self {
            contents: vec![Content {
                parts: vec![Part {
                    text: Some(text.to_string()),
                    inline_data: None,
                }],
            }],
            generation_config: GenerationConfig {
                response_modalities: vec!["AUDIO".to_string()],
                speech_config: Some(SpeechConfig {
                    voice_config: VoiceConfig {
                        prebuilt_voice_config: PrebuiltVoiceConfig {
                            voice_name: voice.to_string(),
                        },
                    },
                }),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub content: Option<Content>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiGenerateContentResponse {
    pub candidates: Option<Vec<Candidate>>,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum ApiError {
    MissingAudio,
    TruncatedAudio { expected_ms: u64, actual_ms: u64 },
    CorruptAudio(String),
    Base64DecodeError(String),
    NetworkError(String),
    ApiServerError(u16, String),
    RateLimited(Option<u64>),
    RateLimitedDaily,
    Unauthorized,
    EmptyResponse,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::MissingAudio => write!(f, "Google Gemini API response did not contain audio payload"),
            ApiError::TruncatedAudio { expected_ms, actual_ms } => write!(f, "Audio appears truncated: expected ~{}ms, got {}ms", expected_ms, actual_ms),
            ApiError::CorruptAudio(msg) => write!(f, "Audio data is corrupt or invalid: {}", msg),
            ApiError::Base64DecodeError(msg) => write!(f, "Failed to decode base64 audio payload: {}", msg),
            ApiError::NetworkError(msg) => write!(f, "Network connectivity error: {}", msg),
            ApiError::ApiServerError(code, msg) => write!(f, "Gemini API error ({}) : {}", code, msg),
            ApiError::RateLimited(retry_after) => write!(f, "Rate limited (429 - Retry after {:?}s)", retry_after),
            ApiError::RateLimitedDaily => write!(f, "Daily API quota exhausted (RPD limit). Resets at midnight Pacific Time."),
            ApiError::Unauthorized => write!(f, "Invalid, missing or unauthorized Gemini API Key"),
            ApiError::EmptyResponse => write!(f, "API returned empty text response"),
        }
    }
}

impl std::error::Error for ApiError {}

pub struct GeminiClient {
    client: reqwest::Client,
}

impl Default for GeminiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .build()
                .unwrap_or_default(),
        }
    }

    /// Dynamic timeout based on text length.
    /// Longer text = longer audio generation = needs more time.
    fn request_timeout_for_text(text: &str) -> std::time::Duration {
        let word_count = text.split_whitespace().count() as u64;
        // ~140 WPM Vietnamese → estimated audio duration
        let estimated_audio_secs = (word_count as f64 / 140.0 * 60.0) as u64;
        let timeout_secs = std::cmp::max(90, estimated_audio_secs * 3 + 30);
        let capped = std::cmp::min(timeout_secs, 300); // Hard cap 5 minutes
        std::time::Duration::from_secs(capped)
    }

    pub fn sanitize_pcm_bytes(raw_bytes: Vec<u8>) -> Vec<u8> {
        // Check for RIFF WAV header
        if raw_bytes.len() > 44 && &raw_bytes[0..4] == b"RIFF" && &raw_bytes[8..12] == b"WAVE" {
            // Scan for the 'data' chunk instead of assuming 44-byte header
            let mut pos = 12; // Skip past "RIFF" + size + "WAVE"
            while pos + 8 <= raw_bytes.len() {
                let chunk_id = &raw_bytes[pos..pos + 4];
                let chunk_size = u32::from_le_bytes([
                    raw_bytes[pos + 4],
                    raw_bytes[pos + 5],
                    raw_bytes[pos + 6],
                    raw_bytes[pos + 7],
                ]) as usize;
                if chunk_id == b"data" {
                    let data_start = pos + 8;
                    let data_end = (data_start + chunk_size).min(raw_bytes.len());
                    info!("Found 'data' chunk at byte offset {}. Extracting {} bytes of PCM.", data_start, data_end - data_start);
                    return raw_bytes[data_start..data_end].to_vec();
                }
                pos += 8 + chunk_size;
                // Ensure 16-bit alignment for RIFF chunks
                if chunk_size % 2 != 0 {
                    pos += 1;
                }
            }
            // Fallback to 44-byte strip if 'data' chunk not found
            info!("RIFF WAV detected but 'data' chunk not found. Falling back to 44-byte header strip.");
            return raw_bytes[44..].to_vec();
        }
        raw_bytes
    }

    pub fn extract_pcm_from_response(response: &GeminiGenerateContentResponse) -> Result<(Vec<u8>, String), ApiError> {
        if let Some(candidates) = &response.candidates {
            for cand in candidates {
                if let Some(content) = &cand.content {
                    for part in &content.parts {
                        if let Some(inline) = &part.inline_data {
                            let mime = inline.mime_type.clone().unwrap_or_else(|| "audio/pcm".to_string());
                            let raw = BASE64_STANDARD
                                .decode(&inline.data)
                                .map_err(|e| ApiError::Base64DecodeError(e.to_string()))?;
                            let pcm = Self::sanitize_pcm_bytes(raw);
                            return Ok((pcm, mime));
                        }
                    }
                }
            }
        }
        Err(ApiError::MissingAudio)
    }

    pub async fn synthesize_speech(
        &self,
        api_key: &str,
        model: &str,
        text: &str,
        voice: &str,
    ) -> Result<Vec<u8>, ApiError> {
        if api_key.trim().is_empty() {
            return Err(ApiError::Unauthorized);
        }

        let clean_key = api_key.trim();
        let target_voice = if voice.trim().is_empty() { "Kore" } else { voice.trim() };

        // IMPORTANT: Use only the requested model. Do NOT fallback to a different model
        // mid-project, as mixing models causes voice character drift between chunks.
        let endpoint = get_generate_content_endpoint(model);
        let payload = GeminiGenerateContentRequest::new_tts_request(text, target_voice);
        let timeout = Self::request_timeout_for_text(text);

        info!("Calling Gemini API [{}] voice [{}] timeout [{}s]...", model, target_voice, timeout.as_secs());

        let res = self
            .client
            .post(&endpoint)
            .header("x-goog-api-key", clean_key)
            .header("Content-Type", "application/json")
            .timeout(timeout)
            .json(&payload)
            .send()
            .await;

        match res {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.json::<GeminiGenerateContentResponse>().await {
                        Ok(parsed_response) => {
                            match Self::extract_pcm_from_response(&parsed_response) {
                                Ok((pcm_bytes, mime)) => {
                                    info!("Synthesized audio OK [{}]. Size: {} bytes, MIME: {}", model, pcm_bytes.len(), mime);
                                    // Basic audio validation: reject suspiciously small output
                                    if pcm_bytes.len() < 1000 {
                                        return Err(ApiError::CorruptAudio(
                                            format!("Audio too small ({} bytes) — likely empty or corrupt", pcm_bytes.len())
                                        ));
                                    }
                                    return Ok(pcm_bytes);
                                }
                                Err(e) => {
                                    warn!("Gemini API [{}] response parsed but no audio data: {}", model, e);
                                    return Err(e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse Gemini [{}] JSON response: {}", model, e);
                            return Err(ApiError::MissingAudio);
                        }
                    }
                } else if status.as_u16() == 401 || status.as_u16() == 403 {
                    return Err(ApiError::Unauthorized);
                } else if status.as_u16() == 429 {
                    // Extract headers BEFORE consuming response body
                    let retry_after = response.headers().get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok());
                    let err_body = response.text().await.unwrap_or_default();
                    // Differentiate RPM vs RPD based on error body
                    let is_daily = err_body.to_lowercase().contains("per day")
                        || err_body.to_lowercase().contains("daily")
                        || err_body.to_lowercase().contains("rpd");
                    if is_daily {
                        warn!("Gemini API [{}] Daily quota exhausted (RPD).", model);
                        return Err(ApiError::RateLimitedDaily);
                    }
                    warn!("Gemini API [{}] Rate Limited (429 RPM). Retry after {:?}s.", model, retry_after);
                    return Err(ApiError::RateLimited(retry_after));
                } else {
                    let err_text = response.text().await.unwrap_or_default();
                    let safe_err = err_text.replace(clean_key, "***REDACTED***");
                    warn!("Gemini API [{}] error ({}): {}", model, status, safe_err);
                    return Err(ApiError::ApiServerError(status.as_u16(), safe_err));
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    warn!("Gemini API [{}] request timed out after {}s.", model, timeout.as_secs());
                    return Err(ApiError::NetworkError(format!("Request timed out after {}s", timeout.as_secs())));
                }
                warn!("Network error calling Gemini API [{}]: {}", model, e);
                return Err(ApiError::NetworkError(e.to_string()));
            }
        }
    }

    pub async fn generate_text(
        &self,
        api_key: &str,
        prompt: &str,
    ) -> Result<String, ApiError> {
        if api_key.trim().is_empty() {
            return Err(ApiError::Unauthorized);
        }

        let endpoint = get_generate_content_endpoint("gemini-2.5-flash");
        let body = serde_json::json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }]
        });

        let response = self
            .client
            .post(&endpoint)
            .header("x-goog-api-key", api_key.trim())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let err_text = response.text().await.unwrap_or_default();
            let safe_err = err_text.replace(api_key.trim(), "***REDACTED***");
            return Err(ApiError::ApiServerError(status.as_u16(), safe_err));
        }

        let val: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        let text = val["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        if text.is_empty() {
            return Err(ApiError::EmptyResponse);
        }

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_request_payload() {
        let req = GeminiGenerateContentRequest::new_tts_request("Xin chào Việt Nam", "Kore");
        let json_str = serde_json::to_string(&req).expect("Failed to serialize request");
        assert!(json_str.contains("Xin chào Việt Nam"));
        assert!(json_str.contains("Kore"));
        assert!(json_str.contains("AUDIO"));
        assert!(json_str.contains("responseModalities"));
        assert!(json_str.contains("prebuiltVoiceConfig"));
    }

    #[test]
    fn test_get_generate_content_endpoint() {
        let endpoint = get_generate_content_endpoint("gemini-3.1-flash-tts-preview");
        assert_eq!(
            endpoint,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-flash-tts-preview:generateContent"
        );
    }

    #[test]
    fn test_extract_pcm_from_valid_response() {
        let raw_pcm = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let b64_data = BASE64_STANDARD.encode(&raw_pcm);

        let response = GeminiGenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    parts: vec![Part {
                        text: None,
                        inline_data: Some(InlineData {
                            mime_type: Some("audio/pcm".to_string()),
                            data: b64_data,
                        }),
                    }],
                }),
                finish_reason: Some("STOP".to_string()),
            }]),
            error: None,
        };

        let (extracted_pcm, mime) = GeminiClient::extract_pcm_from_response(&response)
            .expect("Extraction should succeed for valid audio payload");

        assert_eq!(extracted_pcm, raw_pcm);
        assert_eq!(mime, "audio/pcm");
    }

    #[test]
    fn test_extract_pcm_strips_wav_header_if_present() {
        let mut riff_wav_data = Vec::new();
        riff_wav_data.extend_from_slice(b"RIFF");
        riff_wav_data.extend_from_slice(&(44u32 + 8u32 - 8u32).to_le_bytes());
        riff_wav_data.extend_from_slice(b"WAVE");
        riff_wav_data.extend_from_slice(b"fmt ");
        riff_wav_data.extend_from_slice(&16u32.to_le_bytes());
        riff_wav_data.extend_from_slice(&1u16.to_le_bytes());
        riff_wav_data.extend_from_slice(&1u16.to_le_bytes());
        riff_wav_data.extend_from_slice(&24000u32.to_le_bytes());
        riff_wav_data.extend_from_slice(&(24000u32 * 2).to_le_bytes());
        riff_wav_data.extend_from_slice(&2u16.to_le_bytes());
        riff_wav_data.extend_from_slice(&16u16.to_le_bytes());
        riff_wav_data.extend_from_slice(b"data");
        riff_wav_data.extend_from_slice(&8u32.to_le_bytes());
        let raw_samples = vec![10u8, 20, 30, 40, 50, 60, 70, 80];
        riff_wav_data.extend_from_slice(&raw_samples);

        let b64_data = BASE64_STANDARD.encode(&riff_wav_data);

        let response = GeminiGenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    parts: vec![Part {
                        text: None,
                        inline_data: Some(InlineData {
                            mime_type: Some("audio/wav".to_string()),
                            data: b64_data,
                        }),
                    }],
                }),
                finish_reason: Some("STOP".to_string()),
            }]),
            error: None,
        };

        let (extracted_pcm, mime) = GeminiClient::extract_pcm_from_response(&response)
            .expect("Extraction should succeed and strip header");

        assert_eq!(extracted_pcm, raw_samples);
        assert_eq!(mime, "audio/wav");
    }

    #[test]
    fn test_extract_pcm_missing_audio_returns_error() {
        let response = GeminiGenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(Content {
                    parts: vec![Part {
                        text: Some("Text only response".to_string()),
                        inline_data: None,
                    }],
                }),
                finish_reason: Some("STOP".to_string()),
            }]),
            error: None,
        };

        let res = GeminiClient::extract_pcm_from_response(&response);
        assert!(matches!(res, Err(ApiError::MissingAudio)));
    }

    #[test]
    fn test_api_error_formatting() {
        let err1 = ApiError::RateLimited(Some(60));
        assert!(err1.to_string().contains("429"));
        assert!(err1.to_string().contains("60"));

        let err2 = ApiError::Unauthorized;
        assert!(err2.to_string().contains("unauthorized"));

        let err3 = ApiError::ApiServerError(500, "Internal Server Error".to_string());
        assert!(err3.to_string().contains("500"));
        assert!(err3.to_string().contains("Internal Server Error"));
    }

    #[test]
    fn test_tts_fallback_models_exclude_text_model() {
        let model = DEFAULT_MODEL;
        let mut models_to_try = vec![model];
        if model != FALLBACK_MODEL {
            models_to_try.push(FALLBACK_MODEL);
        }
        assert!(!models_to_try.contains(&"gemini-2.5-flash"));
        assert_eq!(FALLBACK_MODEL, "gemini-2.5-flash-preview-tts");
    }
}

