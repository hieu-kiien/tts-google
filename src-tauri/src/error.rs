use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppErrorResponse {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    AuthInvalid(String),

    #[error("{0}")]
    RateLimited(String),

    #[error("{0}")]
    DailyQuotaExhausted(String),

    #[error("{0}")]
    NetworkUnavailable(String),

    #[error("{0}")]
    ValidationFailed(String),

    #[error("{0}")]
    DatabaseError(String),

    #[error("{0}")]
    AudioCorrupt(String),

    #[error("{0}")]
    FileSystem(String),

    #[error("{0}")]
    Queue(String),

    #[error("{0}")]
    InternalError(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AuthInvalid(_) => "AUTH_INVALID",
            Self::RateLimited(_) => "RATE_LIMITED",
            Self::DailyQuotaExhausted(_) => "DAILY_QUOTA_EXHAUSTED",
            Self::NetworkUnavailable(_) => "NETWORK_UNAVAILABLE",
            Self::ValidationFailed(_) => "VALIDATION_FAILED",
            Self::DatabaseError(_) => "DATABASE_ERROR",
            Self::AudioCorrupt(_) => "AUDIO_CORRUPT",
            Self::FileSystem(_) => "FILE_SYSTEM_ERROR",
            Self::Queue(_) => "QUEUE_ERROR",
            Self::InternalError(_) => "INTERNAL_ERROR",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited(_) | Self::NetworkUnavailable(_) | Self::Queue(_)
        )
    }

    pub fn message(&self) -> String {
        match self {
            Self::AuthInvalid(msg) => msg.clone(),
            Self::RateLimited(msg) => msg.clone(),
            Self::DailyQuotaExhausted(msg) => msg.clone(),
            Self::NetworkUnavailable(msg) => msg.clone(),
            Self::ValidationFailed(msg) => msg.clone(),
            Self::DatabaseError(msg) => msg.clone(),
            Self::AudioCorrupt(msg) => msg.clone(),
            Self::FileSystem(msg) => msg.clone(),
            Self::Queue(msg) => msg.clone(),
            Self::InternalError(msg) => msg.clone(),
        }
    }

    pub fn to_response(&self) -> AppErrorResponse {
        AppErrorResponse {
            code: self.code(),
            message: self.message(),
            retryable: self.is_retryable(),
            diagnostic_id: None,
        }
    }
}

// Tauri requires errors to be Serialize. Serialize as structured object.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_response().serialize(serializer)
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::InternalError(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::InternalError(s.to_string())
    }
}

impl From<crate::api::interactions_client::ApiError> for AppError {
    fn from(e: crate::api::interactions_client::ApiError) -> Self {
        match e {
            crate::api::interactions_client::ApiError::Unauthorized => AppError::AuthInvalid(
                "API Key Gemini không hợp lệ hoặc thiếu quyền truy cập.".to_string(),
            ),
            crate::api::interactions_client::ApiError::NetworkError(msg) => {
                AppError::NetworkUnavailable(format!("Lỗi kết nối mạng: {}", msg))
            }
            crate::api::interactions_client::ApiError::RateLimited(secs) => {
                let wait_str = secs
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "15".to_string());
                AppError::RateLimited(format!(
                    "Vượt quá tần suất yêu cầu API (429). Vui lòng thử lại sau {} giây.",
                    wait_str
                ))
            }
            crate::api::interactions_client::ApiError::RateLimitedDaily => {
                AppError::DailyQuotaExhausted(
                    "Đã chạm hạn ngạch API trong ngày (Daily Quota Exhausted).".to_string(),
                )
            }
            _ => AppError::InternalError(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_serialization_contract() {
        let err = AppError::RateLimited("Rate limit exceeded".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""code":"RATE_LIMITED""#));
        assert!(json.contains(r#""retryable":true"#));
        assert!(json.contains(r#""message":"Rate limit exceeded""#));
    }
}
