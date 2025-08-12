use axum::http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid rating: {0}. Must be between {1} and {2}")]
    InvalidRating(u8, u8, u8),

    #[error("Zone not found: {0}")]
    ZoneNotFound(i64),

    #[error("Instance not found: {0}")]
    InstanceNotFound(i64),

    #[error("Rating not found: {0}")]
    RatingNotFound(i64),

    #[error("Invalid IP hash key length: {0}. Must be at least {1} characters")]
    InvalidIpHashKey(usize, usize),

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::ZoneNotFound(_)
            | AppError::InstanceNotFound(_)
            | AppError::RatingNotFound(_) => StatusCode::NOT_FOUND,
            AppError::InvalidRating(_, _, _) | AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::InvalidIpHashKey(_, _)
            | AppError::MissingEnvVar(_)
            | AppError::InvalidConfig(_)
            | AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Database(_) | AppError::Io(_) | AppError::Json(_) | AppError::Server(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    pub fn log_level(&self) -> tracing::Level {
        match self {
            AppError::ZoneNotFound(_)
            | AppError::InstanceNotFound(_)
            | AppError::RatingNotFound(_) => tracing::Level::DEBUG,
            AppError::InvalidRating(_, _, _) | AppError::Validation(_) => tracing::Level::WARN,
            AppError::InvalidIpHashKey(_, _)
            | AppError::MissingEnvVar(_)
            | AppError::InvalidConfig(_)
            | AppError::Config(_) => tracing::Level::ERROR,
            AppError::Database(_) | AppError::Json(_) | AppError::Server(_) => {
                tracing::Level::ERROR
            }
            AppError::Io(_) => tracing::Level::WARN,
        }
    }
}

// Type alias for Result with AppError
pub type AppResult<T> = Result<T, AppError>;

// Convert AppError to axum response
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let message = self.to_string();

        // Log the error at appropriate level
        match self.log_level() {
            tracing::Level::DEBUG => tracing::debug!("{}", message),
            tracing::Level::WARN => tracing::warn!("{}", message),
            tracing::Level::ERROR => tracing::error!("{}", message),
            _ => tracing::info!("{}", message),
        }

        (status, message).into_response()
    }
}
