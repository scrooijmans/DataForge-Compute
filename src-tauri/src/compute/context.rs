//! Execution context for UDFs.
//!
//! The ExecutionContext provides sandboxed access to input data and parameters
//! during UDF execution. It enforces immutability of inputs and tracks
//! all data access for provenance.

use crate::compute::error::UdfError;
use crate::compute::parameters::ParameterValues;
use crate::compute::types::{CurveData, CurveDataType, InputReference};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Progress callback type for reporting execution progress.
pub type ProgressCallback = Box<dyn Fn(f64, Option<&str>) + Send + Sync>;

/// Shared cancellation token for execution control.
#[derive(Debug, Default)]
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    /// Create a new cancellation token.
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Shared progress state for execution tracking.
#[derive(Debug)]
pub struct ProgressState {
    /// Progress percentage (0-100)
    progress: AtomicU8,
    /// Current status message
    message: std::sync::RwLock<Option<String>>,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            progress: AtomicU8::new(0),
            message: std::sync::RwLock::new(None),
        }
    }
}

impl ProgressState {
    /// Create a new progress state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set progress (0-100).
    pub fn set_progress(&self, percent: f64) {
        let clamped = percent.clamp(0.0, 100.0) as u8;
        self.progress.store(clamped, Ordering::SeqCst);
    }

    /// Set progress with a message.
    pub fn set_progress_with_message(&self, percent: f64, message: impl Into<String>) {
        self.set_progress(percent);
        if let Ok(mut msg) = self.message.write() {
            *msg = Some(message.into());
        }
    }

    /// Get current progress (0-100).
    pub fn get_progress(&self) -> u8 {
        self.progress.load(Ordering::SeqCst)
    }

    /// Get current message.
    pub fn get_message(&self) -> Option<String> {
        self.message.read().ok().and_then(|m| m.clone())
    }
}

/// Execution context providing sandboxed access to data and parameters.
///
/// The context is created by the ExecutionEngine and passed to the UDF
/// during execution. It provides read-only access to input curves and
/// validated parameters.
pub struct ExecutionContext {
    /// Validated parameter values
    parameters: ParameterValues,
    /// Loaded curve data by parameter name
    curves: HashMap<String, Arc<CurveData>>,
    /// Input references for provenance tracking
    input_refs: Vec<InputReference>,
    /// Well ID for the execution (all curves must be from same well)
    well_id: Uuid,
    /// Workspace ID
    workspace_id: Uuid,
    /// Execution metadata
    metadata: HashMap<String, String>,
    /// Cancellation token for cooperative cancellation
    cancellation_token: Arc<CancellationToken>,
    /// Progress state for reporting execution progress
    progress_state: Arc<ProgressState>,
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("well_id", &self.well_id)
            .field("workspace_id", &self.workspace_id)
            .field("curves", &self.curves.keys().collect::<Vec<_>>())
            .field("input_refs", &self.input_refs.len())
            .finish()
    }
}

impl ExecutionContext {
    /// Create a new execution context.
    ///
    /// This is typically called by the ExecutionEngine, not directly by UDFs.
    pub fn new(well_id: Uuid, workspace_id: Uuid, parameters: ParameterValues) -> Self {
        Self {
            parameters,
            curves: HashMap::new(),
            input_refs: Vec::new(),
            well_id,
            workspace_id,
            metadata: HashMap::new(),
            cancellation_token: Arc::new(CancellationToken::new()),
            progress_state: Arc::new(ProgressState::new()),
        }
    }

    /// Create a new execution context with a shared cancellation token.
    pub fn with_cancellation_token(
        well_id: Uuid,
        workspace_id: Uuid,
        parameters: ParameterValues,
        cancellation_token: Arc<CancellationToken>,
    ) -> Self {
        Self {
            parameters,
            curves: HashMap::new(),
            input_refs: Vec::new(),
            well_id,
            workspace_id,
            metadata: HashMap::new(),
            cancellation_token,
            progress_state: Arc::new(ProgressState::new()),
        }
    }

    // === Cancellation Support ===

    /// Check if the execution has been cancelled.
    ///
    /// UDFs should periodically check this and return early if true.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Get the cancellation token for external control.
    pub fn cancellation_token(&self) -> Arc<CancellationToken> {
        self.cancellation_token.clone()
    }

    /// Check cancellation and return an error if cancelled.
    ///
    /// Convenience method for use in UDF loops.
    pub fn check_cancelled(&self) -> Result<(), UdfError> {
        if self.is_cancelled() {
            Err(UdfError::Cancelled)
        } else {
            Ok(())
        }
    }

    // === Progress Reporting ===

    /// Set the current progress (0-100).
    pub fn set_progress(&self, percent: f64) {
        self.progress_state.set_progress(percent);
    }

    /// Set progress with a status message.
    pub fn set_progress_with_message(&self, percent: f64, message: impl Into<String>) {
        self.progress_state.set_progress_with_message(percent, message);
    }

    /// Get the current progress (0-100).
    pub fn get_progress(&self) -> u8 {
        self.progress_state.get_progress()
    }

    /// Get the current progress message.
    pub fn get_progress_message(&self) -> Option<String> {
        self.progress_state.get_message()
    }

    /// Get the progress state for external monitoring.
    pub fn progress_state(&self) -> Arc<ProgressState> {
        self.progress_state.clone()
    }

    /// Get the well ID for this execution.
    pub fn well_id(&self) -> Uuid {
        self.well_id
    }

    /// Get the workspace ID for this execution.
    pub fn workspace_id(&self) -> Uuid {
        self.workspace_id
    }

    /// Get parameter values.
    pub fn parameters(&self) -> &ParameterValues {
        &self.parameters
    }

    /// Get a curve by parameter name.
    ///
    /// Returns the curve data that was bound to the specified parameter.
    pub fn get_curve(&self, param_name: &str) -> Option<Arc<CurveData>> {
        self.curves.get(param_name).cloned()
    }

    /// Get a required curve, returning an error if not found.
    pub fn require_curve(&self, param_name: &str) -> Result<Arc<CurveData>, UdfError> {
        self.get_curve(param_name)
            .ok_or_else(|| UdfError::MissingCurve(param_name.to_string()))
    }

    /// Get all loaded curves.
    pub fn curves(&self) -> &HashMap<String, Arc<CurveData>> {
        &self.curves
    }

    /// Get input references for provenance tracking.
    pub fn input_refs(&self) -> &[InputReference] {
        &self.input_refs
    }

    /// Add a curve to the context.
    ///
    /// This is called by the ExecutionEngine when loading curve data.
    pub fn add_curve(&mut self, param_name: String, curve: Arc<CurveData>) {
        // Track for provenance
        self.input_refs.push(InputReference {
            curve_id: curve.curve_id,
            version: curve.version,
            parquet_hash: curve.parquet_hash.clone(),
        });

        self.curves.insert(param_name, curve);
    }

    /// Set execution metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get execution metadata.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Validate that all curves have compatible depth arrays.
    ///
    /// This checks that all input curves share the same depth values,
    /// which is required for most petrophysical calculations.
    pub fn validate_depth_compatibility(&self) -> Result<(), UdfError> {
        let mut reference_depths: Option<&Arc<Vec<f64>>> = None;

        for (name, curve) in &self.curves {
            match reference_depths {
                None => {
                    reference_depths = Some(&curve.depths);
                }
                Some(ref_depths) => {
                    if !Arc::ptr_eq(ref_depths, &curve.depths) {
                        // Depths are different objects - check if values match
                        if ref_depths.len() != curve.depths.len() {
                            return Err(UdfError::IncompatibleData(format!(
                                "Curve '{}' has {} samples, expected {}",
                                name,
                                curve.depths.len(),
                                ref_depths.len()
                            )));
                        }

                        // Check if depths are approximately equal
                        for (i, (d1, d2)) in
                            ref_depths.iter().zip(curve.depths.iter()).enumerate()
                        {
                            if (d1 - d2).abs() > 1e-6 {
                                return Err(UdfError::IncompatibleData(format!(
                                    "Depth mismatch at index {} for curve '{}': {} vs {}",
                                    i, name, d1, d2
                                )));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the shared depth array from any curve in the context.
    ///
    /// All curves should have compatible depths after validation.
    pub fn get_depths(&self) -> Option<Arc<Vec<f64>>> {
        self.curves.values().next().map(|c| c.depths.clone())
    }

    /// Check if a curve type is present in the context.
    pub fn has_curve_type(&self, curve_type: CurveDataType) -> bool {
        self.curves.values().any(|c| c.curve_type == curve_type)
    }
}

/// Builder for creating execution contexts.
///
/// Used by the ExecutionEngine to construct contexts step by step.
pub struct ExecutionContextBuilder {
    well_id: Uuid,
    workspace_id: Uuid,
    parameters: ParameterValues,
    curves: HashMap<String, Arc<CurveData>>,
    metadata: HashMap<String, String>,
    cancellation_token: Option<Arc<CancellationToken>>,
    progress_state: Option<Arc<ProgressState>>,
}

impl ExecutionContextBuilder {
    /// Create a new builder.
    pub fn new(well_id: Uuid, workspace_id: Uuid) -> Self {
        Self {
            well_id,
            workspace_id,
            parameters: ParameterValues::default(),
            curves: HashMap::new(),
            metadata: HashMap::new(),
            cancellation_token: None,
            progress_state: None,
        }
    }

    /// Set parameter values.
    pub fn with_parameters(mut self, params: ParameterValues) -> Self {
        self.parameters = params;
        self
    }

    /// Add a curve.
    pub fn with_curve(mut self, param_name: impl Into<String>, curve: Arc<CurveData>) -> Self {
        self.curves.insert(param_name.into(), curve);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set a shared cancellation token.
    pub fn with_cancellation_token(mut self, token: Arc<CancellationToken>) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// Set a shared progress state.
    pub fn with_progress_state(mut self, state: Arc<ProgressState>) -> Self {
        self.progress_state = Some(state);
        self
    }

    /// Build the execution context.
    pub fn build(self) -> ExecutionContext {
        let cancellation_token = self
            .cancellation_token
            .unwrap_or_else(|| Arc::new(CancellationToken::new()));
        let progress_state = self
            .progress_state
            .unwrap_or_else(|| Arc::new(ProgressState::new()));

        let mut ctx = ExecutionContext {
            parameters: self.parameters,
            curves: HashMap::new(),
            input_refs: Vec::new(),
            well_id: self.well_id,
            workspace_id: self.workspace_id,
            metadata: self.metadata,
            cancellation_token,
            progress_state,
        };

        for (name, curve) in self.curves {
            ctx.add_curve(name, curve);
        }

        ctx
    }
}
