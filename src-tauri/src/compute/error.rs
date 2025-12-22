//! Error types for the UDF system.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main error type for UDF operations.
#[derive(Debug, Error)]
pub enum UdfError {
    /// Provider is not available (missing dependencies)
    #[error("Provider not available: {0}")]
    ProviderNotAvailable(String),

    /// UDF not found in registry
    #[error("UDF not found: {0}")]
    UdfNotFound(String),

    /// Parameter validation failed
    #[error("Parameter validation failed: {0}")]
    ParameterValidation(String),

    /// Curve type mismatch
    #[error("Curve type mismatch: expected {expected}, got {actual}")]
    CurveTypeMismatch { expected: String, actual: String },

    /// Required curve not provided
    #[error("Required curve not provided: {0}")]
    MissingCurve(String),

    /// Curve data loading failed
    #[error("Failed to load curve data: {0}")]
    CurveLoadError(String),

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Pre-execution check failed
    #[error("Pre-execution check failed: {0}")]
    PreCheckFailed(String),

    /// Post-processing failed
    #[error("Post-processing failed: {0}")]
    PostProcessFailed(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Curve data is incompatible (different depths, lengths, etc.)
    #[error("Incompatible curve data: {0}")]
    IncompatibleData(String),

    /// Numeric computation error (overflow, underflow, NaN)
    #[error("Numeric error: {0}")]
    NumericError(String),
}

impl From<rusqlite::Error> for UdfError {
    fn from(e: rusqlite::Error) -> Self {
        UdfError::DatabaseError(e.to_string())
    }
}

impl From<serde_json::Error> for UdfError {
    fn from(e: serde_json::Error) -> Self {
        UdfError::SerializationError(e.to_string())
    }
}

/// Validation error with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Parameter or input that failed validation
    pub field: String,
    /// Error message
    pub message: String,
    /// Suggested fix (optional)
    pub suggestion: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// Add a suggestion for fixing the error
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, " (suggestion: {})", suggestion)?;
        }
        Ok(())
    }
}
