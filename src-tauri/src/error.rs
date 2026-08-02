use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Audio processing error: {0}")]
    Audio(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("{0}")]
    General(String),
}

// Tauri requires errors to be Serialize
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::General(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::General(s.to_string())
    }
}

impl From<crate::api::interactions_client::ApiError> for AppError {
    fn from(e: crate::api::interactions_client::ApiError) -> Self {
        match e {
            crate::api::interactions_client::ApiError::Unauthorized => {
                AppError::Auth(e.to_string())
            }
            crate::api::interactions_client::ApiError::NetworkError(_) => {
                AppError::Api(e.to_string())
            }
            crate::api::interactions_client::ApiError::RateLimited(_)
            | crate::api::interactions_client::ApiError::RateLimitedDaily => {
                AppError::Api(e.to_string())
            }
            _ => AppError::Api(e.to_string()),
        }
    }
}
