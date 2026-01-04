//! Custom Error Types for ToneForge
//!
//! Provides structured error handling with user-friendly messages.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main error type for ToneForge operations
#[derive(Error, Debug)]
pub enum ToneForgeError {
    #[error("REAPER connection failed: {message}")]
    ReaperConnection { message: String },

    #[error("REAPER operation failed: {operation} - {details}")]
    ReaperOperation { operation: String, details: String },

    #[error("AI provider not configured. Please set your API key first.")]
    AiNotConfigured,

    #[error("AI request failed: {message}")]
    AiRequest { message: String },

    #[error("AI response parsing failed: {message}")]
    AiParsing { message: String },

    #[error("Invalid parameter: {param} - {reason}")]
    InvalidParameter { param: String, reason: String },

    #[error("Track {track} not found")]
    TrackNotFound { track: i32 },

    #[error("FX {fx} not found on track {track}")]
    FxNotFound { track: i32, fx: i32 },

    #[error("Parameter {param} not found on FX {fx}")]
    ParamNotFound { fx: i32, param: String },

    #[error("Preset error: {message}")]
    Preset { message: String },

    #[error("Audio processing error: {message}")]
    AudioProcessing { message: String },

    #[error("File operation failed: {path} - {reason}")]
    FileOperation { path: String, reason: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Undo/Redo error: {message}")]
    UndoRedo { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl ToneForgeError {
    /// Get error code for frontend
    pub fn code(&self) -> &'static str {
        match self {
            ToneForgeError::ReaperConnection { .. } => "REAPER_CONNECTION",
            ToneForgeError::ReaperOperation { .. } => "REAPER_OPERATION",
            ToneForgeError::AiNotConfigured => "AI_NOT_CONFIGURED",
            ToneForgeError::AiRequest { .. } => "AI_REQUEST",
            ToneForgeError::AiParsing { .. } => "AI_PARSING",
            ToneForgeError::InvalidParameter { .. } => "INVALID_PARAMETER",
            ToneForgeError::TrackNotFound { .. } => "TRACK_NOT_FOUND",
            ToneForgeError::FxNotFound { .. } => "FX_NOT_FOUND",
            ToneForgeError::ParamNotFound { .. } => "PARAM_NOT_FOUND",
            ToneForgeError::Preset { .. } => "PRESET_ERROR",
            ToneForgeError::AudioProcessing { .. } => "AUDIO_PROCESSING",
            ToneForgeError::FileOperation { .. } => "FILE_OPERATION",
            ToneForgeError::Network { .. } => "NETWORK_ERROR",
            ToneForgeError::Config { .. } => "CONFIG_ERROR",
            ToneForgeError::UndoRedo { .. } => "UNDO_REDO",
            ToneForgeError::Internal { .. } => "INTERNAL_ERROR",
        }
    }

    /// Get user-friendly suggestion for recovery
    pub fn suggestion(&self) -> &'static str {
        match self {
            ToneForgeError::ReaperConnection { .. } => {
                "Make sure REAPER is running and the ToneForge extension is loaded."
            }
            ToneForgeError::ReaperOperation { .. } => {
                "Try refreshing the connection or restarting REAPER."
            }
            ToneForgeError::AiNotConfigured => {
                "Enter your xAI Grok API key in the sidebar to get started."
            }
            ToneForgeError::AiRequest { .. } => {
                "Check your internet connection and API key validity."
            }
            ToneForgeError::AiParsing { .. } => "Try rephrasing your request more clearly.",
            ToneForgeError::InvalidParameter { .. } => "Check the parameter name and value range.",
            ToneForgeError::TrackNotFound { .. } => "Make sure the track exists in REAPER.",
            ToneForgeError::FxNotFound { .. } => {
                "The plugin may have been removed. Refresh the view."
            }
            ToneForgeError::ParamNotFound { .. } => {
                "The parameter name may be different. Check the plugin."
            }
            ToneForgeError::Preset { .. } => "Check if the preset file exists and is valid.",
            ToneForgeError::AudioProcessing { .. } => {
                "Check if the audio file format is supported."
            }
            ToneForgeError::FileOperation { .. } => "Check file permissions and path validity.",
            ToneForgeError::Network { .. } => "Check your internet connection.",
            ToneForgeError::Config { .. } => "Check your configuration settings.",
            ToneForgeError::UndoRedo { .. } => "The undo history may be corrupted. Try again.",
            ToneForgeError::Internal { .. } => "Please report this issue if it persists.",
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ToneForgeError::Network { .. }
                | ToneForgeError::AiRequest { .. }
                | ToneForgeError::ReaperConnection { .. }
        )
    }
}

/// Serializable error response for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub suggestion: String,
    pub recoverable: bool,
}

impl From<ToneForgeError> for ErrorResponse {
    fn from(err: ToneForgeError) -> Self {
        Self {
            code: err.code().to_string(),
            message: err.to_string(),
            suggestion: err.suggestion().to_string(),
            recoverable: err.is_recoverable(),
        }
    }
}

impl From<ToneForgeError> for String {
    fn from(err: ToneForgeError) -> Self {
        let fallback = err.to_string();
        serde_json::to_string(&ErrorResponse::from(err)).unwrap_or(fallback)
    }
}

/// Result type alias for ToneForge operations
pub type ToneForgeResult<T> = Result<T, ToneForgeError>;

/// Convert various error types to ToneForgeError
impl From<reqwest::Error> for ToneForgeError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() {
            ToneForgeError::ReaperConnection {
                message: "Could not connect to REAPER".to_string(),
            }
        } else if err.is_timeout() {
            ToneForgeError::Network {
                message: "Request timed out".to_string(),
            }
        } else {
            ToneForgeError::Network {
                message: err.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for ToneForgeError {
    fn from(err: serde_json::Error) -> Self {
        ToneForgeError::AiParsing {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ToneForgeError {
    fn from(err: std::io::Error) -> Self {
        ToneForgeError::FileOperation {
            path: "unknown".to_string(),
            reason: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = ToneForgeError::ReaperConnection {
            message: "test".to_string(),
        };
        assert_eq!(err.code(), "REAPER_CONNECTION");
    }

    #[test]
    fn test_error_response_serialization() {
        let err = ToneForgeError::AiNotConfigured;
        let response: ErrorResponse = err.into();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("AI_NOT_CONFIGURED"));
    }

    #[test]
    fn test_recoverable_errors() {
        assert!(ToneForgeError::Network {
            message: "test".to_string()
        }
        .is_recoverable());
        assert!(!ToneForgeError::Internal {
            message: "test".to_string()
        }
        .is_recoverable());
    }
}
