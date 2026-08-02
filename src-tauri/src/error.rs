use serde::{Deserialize, Serialize};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppErrorCode {
    AuthInvalid,
    RateLimited,
    DailyQuotaExhausted,
    NetworkUnavailable,
    ValidationFailed,
    DatabaseError,
    AudioCorrupt,
    ContentFiltered,
    FileSystemError,
    QueueError,
    InternalError,
}

impl AppErrorCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AuthInvalid => "AUTH_INVALID",
            Self::RateLimited => "RATE_LIMITED",
            Self::DailyQuotaExhausted => "DAILY_QUOTA_EXHAUSTED",
            Self::NetworkUnavailable => "NETWORK_UNAVAILABLE",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::DatabaseError => "DATABASE_ERROR",
            Self::AudioCorrupt => "AUDIO_CORRUPT",
            Self::ContentFiltered => "CONTENT_FILTERED",
            Self::FileSystemError => "FILE_SYSTEM_ERROR",
            Self::QueueError => "QUEUE_ERROR",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }
}

impl std::str::FromStr for AppErrorCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AUTH_INVALID" | "401_UNAUTHORIZED" => Ok(Self::AuthInvalid),
            "RATE_LIMITED" | "429_RATE_LIMITED" => Ok(Self::RateLimited),
            "DAILY_QUOTA_EXHAUSTED" | "429_QUOTA_EXHAUSTED" => Ok(Self::DailyQuotaExhausted),
            "NETWORK_UNAVAILABLE" => Ok(Self::NetworkUnavailable),
            "VALIDATION_FAILED" => Ok(Self::ValidationFailed),
            "DATABASE_ERROR" => Ok(Self::DatabaseError),
            "AUDIO_CORRUPT" | "WAV_WRITE_ERROR" => Ok(Self::AudioCorrupt),
            "CONTENT_FILTERED" | "400_CONTENT_FILTERED" => Ok(Self::ContentFiltered),
            "FILE_SYSTEM_ERROR" => Ok(Self::FileSystemError),
            "QUEUE_ERROR" => Ok(Self::QueueError),
            "INTERNAL_ERROR" | "API_ERROR" => Ok(Self::InternalError),
            _ => Err(()),
        }
    }
}

impl rusqlite::types::ToSql for AppErrorCode {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}

impl rusqlite::types::FromSql for AppErrorCode {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let text = value.as_str()?;
        text.parse::<Self>().map_err(|_| {
            rusqlite::types::FromSqlError::Other(format!("Invalid AppErrorCode: {}", text).into())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppErrorResponse {
    pub code: AppErrorCode,
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
    ContentFiltered(String),

    #[error("{0}")]
    FileSystem(String),

    #[error("{0}")]
    Queue(String),

    #[error("{0}")]
    InternalError(String),
}

impl AppError {
    pub fn code(&self) -> AppErrorCode {
        match self {
            Self::AuthInvalid(_) => AppErrorCode::AuthInvalid,
            Self::RateLimited(_) => AppErrorCode::RateLimited,
            Self::DailyQuotaExhausted(_) => AppErrorCode::DailyQuotaExhausted,
            Self::NetworkUnavailable(_) => AppErrorCode::NetworkUnavailable,
            Self::ValidationFailed(_) => AppErrorCode::ValidationFailed,
            Self::DatabaseError(_) => AppErrorCode::DatabaseError,
            Self::AudioCorrupt(_) => AppErrorCode::AudioCorrupt,
            Self::ContentFiltered(_) => AppErrorCode::ContentFiltered,
            Self::FileSystem(_) => AppErrorCode::FileSystemError,
            Self::Queue(_) => AppErrorCode::QueueError,
            Self::InternalError(_) => AppErrorCode::InternalError,
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
            Self::ContentFiltered(msg) => msg.clone(),
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
