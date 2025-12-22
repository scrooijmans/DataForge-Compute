//! Execution engine for running UDFs.
//!
//! The ExecutionEngine orchestrates UDF execution with multi-stage validation:
//! 1. GUI validation (frontend - not in this module)
//! 2. Parameter definition validation
//! 3. UDF-level validation (check_parameters)
//! 4. Pre-execution hook (prepare)
//! 5. Execution
//! 6. Post-execution hook (postprocess)

use crate::compute::context::{ExecutionContext, ExecutionContextBuilder};
use crate::compute::error::{UdfError, ValidationError};
use crate::compute::parameters::{CurveParameter, ParameterDefinition, ParameterValue, ParameterValues};
use crate::compute::registry::UdfRegistry;
use crate::compute::types::{
    CurveData, CurveDataType, ExecutionRecord, ExecutionStatus, UdfOutput,
};
use crate::compute::Udf;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Result of UDF execution.
#[derive(Debug)]
pub struct ExecutionResult {
    /// The execution record for provenance
    pub record: ExecutionRecord,
    /// The output if successful
    pub output: Option<UdfOutput>,
}

/// Execution engine for running UDFs with validation.
pub struct ExecutionEngine {
    /// Reference to the UDF registry
    registry: Arc<UdfRegistry>,
    /// Application version for provenance
    app_version: String,
}

impl ExecutionEngine {
    /// Create a new execution engine.
    pub fn new(registry: Arc<UdfRegistry>, app_version: impl Into<String>) -> Self {
        Self {
            registry,
            app_version: app_version.into(),
        }
    }

    /// Execute a UDF with the given parameters and curve data.
    ///
    /// This is the main entry point for UDF execution. It performs
    /// multi-stage validation and tracks provenance.
    pub fn execute(
        &self,
        udf_id: &str,
        well_id: Uuid,
        workspace_id: Uuid,
        parameters: HashMap<String, ParameterValue>,
        curve_loader: &dyn CurveLoader,
    ) -> Result<ExecutionResult, UdfError> {
        let started_at = Utc::now();

        // Get the UDF
        let udf = self
            .registry
            .get_udf(udf_id)
            .ok_or_else(|| UdfError::UdfNotFound(udf_id.to_string()))?;

        let metadata = udf.metadata();

        // Create initial execution record
        let mut record = ExecutionRecord {
            id: Uuid::new_v4(),
            udf_id: udf_id.to_string(),
            udf_version: metadata.version.clone(),
            inputs: Vec::new(),
            parameters: serde_json::to_value(&parameters).unwrap_or(serde_json::Value::Null),
            output_curve_id: None,
            output_parquet_hash: None,
            started_at,
            completed_at: None,
            compute_app_version: self.app_version.clone(),
            status: ExecutionStatus::Failed,
            error_message: None,
        };

        // Execute with error handling
        match self.execute_inner(&udf, well_id, workspace_id, parameters, curve_loader) {
            Ok((mut context, mut output)) => {
                // Post-process
                if let Err(e) = udf.postprocess(&mut output, &context) {
                    record.completed_at = Some(Utc::now());
                    record.error_message = Some(e.to_string());
                    return Ok(ExecutionResult {
                        record,
                        output: None,
                    });
                }

                // Update record with success info
                record.inputs = context.input_refs().to_vec();
                record.status = ExecutionStatus::Completed;
                record.completed_at = Some(Utc::now());

                Ok(ExecutionResult {
                    record,
                    output: Some(output),
                })
            }
            Err(e) => {
                record.completed_at = Some(Utc::now());
                record.error_message = Some(e.to_string());
                Ok(ExecutionResult {
                    record,
                    output: None,
                })
            }
        }
    }

    /// Inner execution with validation stages.
    fn execute_inner(
        &self,
        udf: &Arc<dyn Udf>,
        well_id: Uuid,
        workspace_id: Uuid,
        parameters: HashMap<String, ParameterValue>,
        curve_loader: &dyn CurveLoader,
    ) -> Result<(ExecutionContext, UdfOutput), UdfError> {
        let param_defs = udf.parameter_definitions();
        let param_values = ParameterValues::from_map(parameters.clone());

        // Stage 1: Parameter definition validation
        let validation_errors = self.validate_parameters(&param_defs, &parameters)?;
        if !validation_errors.is_empty() {
            return Err(UdfError::ParameterValidation(
                validation_errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }

        // Stage 2: Load curve data and validate types
        let mut context_builder = ExecutionContextBuilder::new(well_id, workspace_id)
            .with_parameters(param_values);

        for def in &param_defs {
            if def.param_type() == "curve" {
                if let Some(value) = parameters.get(def.name()) {
                    if let Some(curve_id) = value.as_curve() {
                        // Load the curve
                        let curve = curve_loader.load_curve(curve_id)?;

                        // Validate curve type if this is a CurveParameter
                        // We need to downcast to check allowed_types
                        self.validate_curve_type(def.as_ref(), &curve)?;

                        context_builder = context_builder.with_curve(def.name(), curve);
                    }
                }
            }
        }

        let mut context = context_builder.build();

        // Stage 3: Validate depth compatibility
        context.validate_depth_compatibility()?;

        // Stage 4: UDF-level parameter validation
        if let Err(errors) = udf.check_parameters(&context) {
            return Err(UdfError::ParameterValidation(
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }

        // Stage 5: Check if UDF can execute
        if !udf.can_execute(&context) {
            return Err(UdfError::PreCheckFailed(
                "UDF cannot execute in current context".to_string(),
            ));
        }

        // Stage 6: Pre-execution hook
        if !udf.prepare(&mut context)? {
            return Err(UdfError::PreCheckFailed(
                "Pre-execution check failed".to_string(),
            ));
        }

        // Stage 7: Execute
        let output = udf.execute(&context)?;

        Ok((context, output))
    }

    /// Validate parameters against their definitions.
    fn validate_parameters(
        &self,
        definitions: &[Box<dyn ParameterDefinition>],
        values: &HashMap<String, ParameterValue>,
    ) -> Result<Vec<ValidationError>, UdfError> {
        let mut errors = Vec::new();

        for def in definitions {
            let value = values
                .get(def.name())
                .cloned()
                .unwrap_or(ParameterValue::Null);

            // Check if required and missing
            if value.is_null() && def.is_required() && def.default_value().is_none() {
                errors.push(ValidationError::new(
                    def.name(),
                    format!("'{}' is required", def.label()),
                ));
                continue;
            }

            // Apply default if null
            let final_value = if value.is_null() {
                def.default_value().unwrap_or(ParameterValue::Null)
            } else {
                value
            };

            // Validate against definition
            if let Err(e) = def.validate(&final_value) {
                errors.push(e);
            }
        }

        Ok(errors)
    }

    /// Validate that a curve matches the type constraints of a curve parameter.
    fn validate_curve_type(
        &self,
        def: &dyn ParameterDefinition,
        curve: &CurveData,
    ) -> Result<(), UdfError> {
        // This is a bit of a hack since we can't downcast trait objects easily.
        // The to_json() method includes allowed_types, so we can check there.
        let json = def.to_json();

        if let Some(allowed_types) = json.get("allowed_types").and_then(|v| v.as_array()) {
            if allowed_types.is_empty() {
                return Ok(()); // Any type allowed
            }

            let curve_type_name = curve.curve_type.display_name();
            let is_allowed = allowed_types
                .iter()
                .filter_map(|v| v.as_str())
                .any(|t| t == curve_type_name);

            if !is_allowed {
                let allowed_str = allowed_types
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                return Err(UdfError::CurveTypeMismatch {
                    expected: allowed_str,
                    actual: curve_type_name.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate parameters without executing (for UI feedback).
    pub fn validate_only(
        &self,
        udf_id: &str,
        parameters: &HashMap<String, ParameterValue>,
    ) -> Result<Vec<ValidationError>, UdfError> {
        let udf = self
            .registry
            .get_udf(udf_id)
            .ok_or_else(|| UdfError::UdfNotFound(udf_id.to_string()))?;

        let param_defs = udf.parameter_definitions();
        self.validate_parameters(&param_defs, parameters)
    }

    /// Get parameter definitions for a UDF.
    pub fn get_parameter_definitions(
        &self,
        udf_id: &str,
    ) -> Result<Vec<serde_json::Value>, UdfError> {
        let udf = self
            .registry
            .get_udf(udf_id)
            .ok_or_else(|| UdfError::UdfNotFound(udf_id.to_string()))?;

        Ok(udf.parameter_definitions().iter().map(|d| d.to_json()).collect())
    }
}

/// Trait for loading curve data.
///
/// This abstracts the data access layer so the engine can be tested
/// without a real database.
pub trait CurveLoader {
    /// Load curve data by ID.
    fn load_curve(&self, curve_id: Uuid) -> Result<Arc<CurveData>, UdfError>;

    /// Load curve metadata (type, unit, etc.) without loading values.
    fn load_curve_metadata(&self, curve_id: Uuid) -> Result<CurveMetadataInfo, UdfError>;
}

/// Minimal curve metadata for validation.
#[derive(Debug, Clone)]
pub struct CurveMetadataInfo {
    pub curve_id: Uuid,
    pub mnemonic: String,
    pub curve_type: CurveDataType,
    pub unit: String,
    pub row_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here with mock CurveLoader implementations
}
