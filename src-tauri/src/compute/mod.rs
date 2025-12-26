//! DataForge Compute - UDF-based computation engine
//!
//! This module implements the User-Defined Function (UDF) architecture
//! for running domain computations on well log curve data.
//!
//! # Architecture
//!
//! - **Provider-based extensibility**: UDFs are grouped into providers (like QGIS plugins)
//! - **Explicit execution only**: Tools run only when user clicks "Execute"
//! - **Immutable inputs**: UDF inputs are read-only curve data
//! - **Append-only outputs**: Derived curves are new artifacts with provenance
//! - **Type-safe curve parameters**: UDFs declare which curve types they accept

pub mod context;
pub mod data_loader;
pub mod engine;
pub mod error;
pub mod output_writer;
pub mod parameters;
pub mod providers;
pub mod registry;
pub mod types;

// Re-export main types
pub use context::{CancellationToken, ExecutionContext, ProgressState};
pub use data_loader::{DataForgeCurveLoader, init_compute_schema, save_execution_record};
pub use engine::ExecutionEngine;
pub use error::{UdfError, ValidationError};
pub use parameters::{CurveParameter, NumericParameter, ParameterDefinition, ParameterValue};
pub use registry::UdfRegistry;
pub use types::{
    CurveData, CurveDataType, ExecutionRecord, ExecutionStatus, InputReference, UdfMetadata,
    UdfOutput,
};

use std::sync::Arc;

/// Core trait for UDF providers.
///
/// A provider groups related UDFs together (e.g., "petro" provider for petrophysics).
/// Providers are loaded at startup and register their UDFs with the registry.
pub trait UdfProvider: Send + Sync {
    /// Unique identifier for this provider (e.g., "petro", "qc", "transforms")
    fn id(&self) -> &str;

    /// Human-readable display name
    fn name(&self) -> &str;

    /// Provider version (semver)
    fn version(&self) -> &str;

    /// Description of what this provider offers
    fn description(&self) -> &str;

    /// Load all UDFs from this provider
    fn load_udfs(&self) -> Vec<Arc<dyn Udf>>;

    /// Check if provider dependencies are available
    fn is_available(&self) -> Result<(), UdfError> {
        Ok(())
    }
}

/// Core trait for User-Defined Functions.
///
/// UDFs are the fundamental unit of computation in DataForge Compute.
/// Each UDF declares its inputs, parameters, and output schema, then
/// implements the `execute` method to perform the computation.
pub trait Udf: Send + Sync {
    /// Unique identifier within the provider (e.g., "vshale_linear")
    /// Full ID is "provider:udf_id" (e.g., "petro:vshale_linear")
    fn id(&self) -> &str;

    /// Metadata for display and documentation
    fn metadata(&self) -> UdfMetadata;

    /// Parameter definitions for this UDF
    fn parameter_definitions(&self) -> Vec<Box<dyn ParameterDefinition>>;

    /// Check if this UDF can execute given the current context.
    /// Used for dynamic availability (e.g., based on data state).
    fn can_execute(&self, context: &ExecutionContext) -> bool {
        let _ = context;
        true
    }

    /// Pre-execution hook for setup/validation.
    /// Return false to abort execution.
    fn prepare(&self, context: &mut ExecutionContext) -> Result<bool, UdfError> {
        let _ = context;
        Ok(true)
    }

    /// Execute the UDF with the given context.
    /// This is the main computation method.
    fn execute(&self, context: &ExecutionContext) -> Result<UdfOutput, UdfError>;

    /// Post-execution hook for cleanup or additional processing.
    fn postprocess(&self, output: &mut UdfOutput, context: &ExecutionContext) -> Result<(), UdfError> {
        let _ = (output, context);
        Ok(())
    }

    /// Validate parameters beyond basic type checking.
    /// Called before prepare().
    fn check_parameters(&self, context: &ExecutionContext) -> Result<(), Vec<ValidationError>> {
        let _ = context;
        Ok(())
    }
}
