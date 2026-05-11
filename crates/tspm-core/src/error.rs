use thiserror::Error;

/// TSPM error type
#[derive(Debug, Error)]
pub enum TspmError {
    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: String },

    #[error("Failed to read config file {path}: {source}")]
    ConfigRead {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse config file {path}: {source}")]
    ConfigParse {
        path: String,
        source: toml::de::Error,
    },

    #[error("Configuration validation failed: {message}")]
    ConfigValidation { message: String },

    #[error("Process not found: {name}")]
    ProcessNotFound { name: String },

    #[error("Process '{name}' failed to start: {reason}")]
    ProcessStartFailed { name: String, reason: String },

    #[error("Process '{name}' is not running")]
    ProcessNotRunning { name: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type TspmResult<T> = Result<T, TspmError>;
