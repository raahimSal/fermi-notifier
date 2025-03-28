use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] std::env::VarError),
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON serialization/deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Gemini API error: {0}")]
    GeminiApi(String),
    #[error("ntfy.sh notification error: {0}")]
    Ntfy(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse content: {0}")]
    ParseError(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

pub type AppResult<T> = Result<T, AppError>;
