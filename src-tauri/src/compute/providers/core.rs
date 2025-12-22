//! Core UDF provider with fundamental data processing tools.
//!
//! This provider contains basic, verifiable computations that are
//! commonly used in well log data processing.

use crate::compute::context::ExecutionContext;
use crate::compute::error::UdfError;
use crate::compute::parameters::{CurveParameter, NumericParameter, ParameterDefinition};
use crate::compute::types::{CurveDataType, OutputCurveData, UdfMetadata, UdfOutput};
use crate::compute::{Udf, UdfProvider};
use std::sync::Arc;

/// Core provider with fundamental data processing tools.
pub struct CoreProvider {
    version: String,
}

impl CoreProvider {
    pub fn new() -> Self {
        Self {
            version: "0.1.0".to_string(),
        }
    }
}

impl Default for CoreProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UdfProvider for CoreProvider {
    fn id(&self) -> &str {
        "core"
    }

    fn name(&self) -> &str {
        "Core Processing"
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        "Fundamental data processing tools for well log curves"
    }

    fn load_udfs(&self) -> Vec<Arc<dyn Udf>> {
        vec![
            Arc::new(MovingAverageUdf::new()),
            Arc::new(LinearScaleUdf::new()),
            Arc::new(DepthResampleUdf::new()),
        ]
    }
}

// =============================================================================
// Moving Average UDF
// =============================================================================

/// Moving average smoothing filter.
///
/// Applies a centered moving average window to smooth curve data.
/// Handles null values by excluding them from the average calculation.
pub struct MovingAverageUdf;

impl MovingAverageUdf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MovingAverageUdf {
    fn default() -> Self {
        Self::new()
    }
}

impl Udf for MovingAverageUdf {
    fn id(&self) -> &str {
        "moving_average"
    }

    fn metadata(&self) -> UdfMetadata {
        UdfMetadata {
            name: "Moving Average".to_string(),
            category: "Smoothing".to_string(),
            description: "Apply a centered moving average filter to smooth curve data".to_string(),
            documentation: Some(
                r#"# Moving Average

Applies a centered moving average window to smooth curve data.

## Parameters

- **Input Curve**: Any numeric curve to smooth
- **Window Size**: Number of samples in the averaging window (odd values recommended)

## Algorithm

For each sample at position i:
```
smoothed[i] = mean(values[i - window/2 : i + window/2 + 1])
```

Null values are excluded from the average calculation.

## Output

- Smoothed curve with same length as input
- Edge samples use smaller windows (partial averaging)
"#
                .to_string(),
            ),
            version: "1.0.0".to_string(),
            tags: vec![
                "smooth".to_string(),
                "filter".to_string(),
                "average".to_string(),
                "noise".to_string(),
            ],
        }
    }

    fn parameter_definitions(&self) -> Vec<Box<dyn ParameterDefinition>> {
        vec![
            Box::new(
                CurveParameter::required("input_curve", "Input Curve")
                    .with_description("Curve to apply moving average smoothing"),
            ),
            Box::new(
                NumericParameter::optional("window_size", "Window Size", 5.0)
                    .with_description("Number of samples in the averaging window")
                    .with_range(3.0, 101.0),
            ),
        ]
    }

    fn check_parameters(&self, context: &ExecutionContext) -> Result<(), Vec<crate::compute::ValidationError>> {
        let params = context.parameters();
        let mut errors = Vec::new();

        let window = params.get_f64("window_size").unwrap_or(5.0) as usize;

        if window % 2 == 0 {
            errors.push(
                crate::compute::ValidationError::new(
                    "window_size",
                    "Window size should be odd for symmetric averaging",
                )
                .with_suggestion("Use an odd number like 3, 5, 7, etc."),
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn execute(&self, context: &ExecutionContext) -> Result<UdfOutput, UdfError> {
        let input_curve = context.require_curve("input_curve")?;
        let params = context.parameters();
        let window_size = params.get_f64("window_size").unwrap_or(5.0) as usize;

        let half_window = window_size / 2;
        let mut smoothed_values: Vec<Option<f64>> = Vec::with_capacity(input_curve.len());

        for i in 0..input_curve.len() {
            let start = i.saturating_sub(half_window);
            let end = (i + half_window + 1).min(input_curve.len());

            // Collect valid values in window
            let window_values: Vec<f64> = input_curve.values[start..end]
                .iter()
                .filter_map(|v| *v)
                .collect();

            if window_values.is_empty() {
                smoothed_values.push(None);
            } else {
                let avg = window_values.iter().sum::<f64>() / window_values.len() as f64;
                smoothed_values.push(Some(avg));
            }
        }

        let output_curve = OutputCurveData {
            mnemonic: format!("{}_MA{}", input_curve.mnemonic, window_size),
            curve_type: input_curve.curve_type,
            unit: input_curve.unit.clone(),
            depths: input_curve.depths.as_ref().clone(),
            values: smoothed_values,
            description: Some(format!(
                "Moving average (window={}) of {}",
                window_size, input_curve.mnemonic
            )),
        };

        let mut output = UdfOutput::new(output_curve);
        output.add_metadata("window_size", serde_json::json!(window_size));
        output.add_metadata("input_curve", serde_json::json!(input_curve.mnemonic));

        Ok(output)
    }
}

// =============================================================================
// Linear Scale UDF
// =============================================================================

/// Linear scaling/normalization of curve data.
///
/// Applies a linear transformation: output = (input - in_min) / (in_max - in_min) * (out_max - out_min) + out_min
pub struct LinearScaleUdf;

impl LinearScaleUdf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinearScaleUdf {
    fn default() -> Self {
        Self::new()
    }
}

impl Udf for LinearScaleUdf {
    fn id(&self) -> &str {
        "linear_scale"
    }

    fn metadata(&self) -> UdfMetadata {
        UdfMetadata {
            name: "Linear Scale".to_string(),
            category: "Transform".to_string(),
            description: "Apply linear scaling/normalization to curve data".to_string(),
            documentation: Some(
                r#"# Linear Scale

Applies a linear transformation to scale curve values from one range to another.

## Formula

```
output = (input - in_min) / (in_max - in_min) * (out_max - out_min) + out_min
```

## Common Uses

- Normalize data to 0-1 range
- Convert units (when linear relationship exists)
- Rescale for visualization

## Parameters

- **Input Range**: The min/max of the input data range
- **Output Range**: The desired min/max of the output data range
"#
                .to_string(),
            ),
            version: "1.0.0".to_string(),
            tags: vec![
                "scale".to_string(),
                "normalize".to_string(),
                "transform".to_string(),
                "linear".to_string(),
            ],
        }
    }

    fn parameter_definitions(&self) -> Vec<Box<dyn ParameterDefinition>> {
        vec![
            Box::new(
                CurveParameter::required("input_curve", "Input Curve")
                    .with_description("Curve to scale"),
            ),
            Box::new(
                NumericParameter::required("in_min", "Input Min")
                    .with_description("Minimum value of input range"),
            ),
            Box::new(
                NumericParameter::required("in_max", "Input Max")
                    .with_description("Maximum value of input range"),
            ),
            Box::new(
                NumericParameter::optional("out_min", "Output Min", 0.0)
                    .with_description("Minimum value of output range"),
            ),
            Box::new(
                NumericParameter::optional("out_max", "Output Max", 1.0)
                    .with_description("Maximum value of output range"),
            ),
        ]
    }

    fn check_parameters(&self, context: &ExecutionContext) -> Result<(), Vec<crate::compute::ValidationError>> {
        let params = context.parameters();
        let mut errors = Vec::new();

        let in_min = params.get_f64("in_min").unwrap_or(0.0);
        let in_max = params.get_f64("in_max").unwrap_or(1.0);

        if (in_max - in_min).abs() < 1e-10 {
            errors.push(crate::compute::ValidationError::new(
                "in_max",
                "Input range cannot be zero (in_min == in_max)",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn execute(&self, context: &ExecutionContext) -> Result<UdfOutput, UdfError> {
        let input_curve = context.require_curve("input_curve")?;
        let params = context.parameters();

        let in_min = params.get_f64("in_min").ok_or_else(|| {
            UdfError::ParameterValidation("in_min is required".to_string())
        })?;
        let in_max = params.get_f64("in_max").ok_or_else(|| {
            UdfError::ParameterValidation("in_max is required".to_string())
        })?;
        let out_min = params.get_f64_or("out_min", 0.0);
        let out_max = params.get_f64_or("out_max", 1.0);

        let in_range = in_max - in_min;
        let out_range = out_max - out_min;

        let scaled_values: Vec<Option<f64>> = input_curve
            .values
            .iter()
            .map(|v| {
                v.map(|val| {
                    let normalized = (val - in_min) / in_range;
                    normalized * out_range + out_min
                })
            })
            .collect();

        let output_curve = OutputCurveData {
            mnemonic: format!("{}_SCALED", input_curve.mnemonic),
            curve_type: CurveDataType::Computed,
            unit: "".to_string(), // Unit changes after scaling
            depths: input_curve.depths.as_ref().clone(),
            values: scaled_values,
            description: Some(format!(
                "Linear scale of {} from [{:.2}, {:.2}] to [{:.2}, {:.2}]",
                input_curve.mnemonic, in_min, in_max, out_min, out_max
            )),
        };

        let mut output = UdfOutput::new(output_curve);
        output.add_metadata("in_min", serde_json::json!(in_min));
        output.add_metadata("in_max", serde_json::json!(in_max));
        output.add_metadata("out_min", serde_json::json!(out_min));
        output.add_metadata("out_max", serde_json::json!(out_max));

        Ok(output)
    }
}

// =============================================================================
// Depth Resample UDF
// =============================================================================

/// Resample curve data to a new depth interval.
///
/// Interpolates curve values to match a new regular depth spacing.
pub struct DepthResampleUdf;

impl DepthResampleUdf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DepthResampleUdf {
    fn default() -> Self {
        Self::new()
    }
}

impl Udf for DepthResampleUdf {
    fn id(&self) -> &str {
        "depth_resample"
    }

    fn metadata(&self) -> UdfMetadata {
        UdfMetadata {
            name: "Depth Resample".to_string(),
            category: "Transform".to_string(),
            description: "Resample curve data to a new depth interval".to_string(),
            documentation: Some(
                r#"# Depth Resample

Resamples curve data to a new regular depth spacing using linear interpolation.

## Parameters

- **Input Curve**: Curve to resample
- **New Step**: Desired depth interval (e.g., 0.5 for half-foot sampling)
- **Start Depth**: Optional start depth (defaults to first sample)
- **End Depth**: Optional end depth (defaults to last sample)

## Algorithm

Uses linear interpolation between adjacent samples to compute values at new depths.
Extrapolation beyond the original depth range is not performed (returns null).

## Output

- Curve with regular depth spacing
- Values linearly interpolated from input
"#
                .to_string(),
            ),
            version: "1.0.0".to_string(),
            tags: vec![
                "resample".to_string(),
                "depth".to_string(),
                "interpolate".to_string(),
                "spacing".to_string(),
            ],
        }
    }

    fn parameter_definitions(&self) -> Vec<Box<dyn ParameterDefinition>> {
        vec![
            Box::new(
                CurveParameter::required("input_curve", "Input Curve")
                    .with_description("Curve to resample"),
            ),
            Box::new(
                NumericParameter::required("new_step", "New Step")
                    .with_description("New depth interval (same units as input)")
                    .with_min(0.001),
            ),
            Box::new(
                NumericParameter::optional("start_depth", "Start Depth", f64::NAN)
                    .with_description("Start depth (leave empty to use first sample)"),
            ),
            Box::new(
                NumericParameter::optional("end_depth", "End Depth", f64::NAN)
                    .with_description("End depth (leave empty to use last sample)"),
            ),
        ]
    }

    fn check_parameters(&self, context: &ExecutionContext) -> Result<(), Vec<crate::compute::ValidationError>> {
        let params = context.parameters();
        let mut errors = Vec::new();

        let new_step = params.get_f64("new_step").unwrap_or(0.0);
        if new_step <= 0.0 {
            errors.push(crate::compute::ValidationError::new(
                "new_step",
                "Step size must be positive",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn execute(&self, context: &ExecutionContext) -> Result<UdfOutput, UdfError> {
        let input_curve = context.require_curve("input_curve")?;
        let params = context.parameters();

        let new_step = params.get_f64("new_step").ok_or_else(|| {
            UdfError::ParameterValidation("new_step is required".to_string())
        })?;

        // Determine depth range
        let original_depths = &input_curve.depths;
        if original_depths.is_empty() {
            return Err(UdfError::ExecutionFailed("Input curve has no data".to_string()));
        }

        let orig_start = original_depths.first().copied().unwrap_or(0.0);
        let orig_end = original_depths.last().copied().unwrap_or(0.0);

        let start_depth = params.get_f64("start_depth").filter(|v| !v.is_nan()).unwrap_or(orig_start);
        let end_depth = params.get_f64("end_depth").filter(|v| !v.is_nan()).unwrap_or(orig_end);

        if end_depth <= start_depth {
            return Err(UdfError::ParameterValidation(
                "End depth must be greater than start depth".to_string(),
            ));
        }

        // Generate new depth array
        let mut new_depths: Vec<f64> = Vec::new();
        let mut depth = start_depth;
        while depth <= end_depth + 1e-10 {
            new_depths.push(depth);
            depth += new_step;
        }

        // Interpolate values
        let mut new_values: Vec<Option<f64>> = Vec::with_capacity(new_depths.len());

        for &target_depth in &new_depths {
            // Find bracketing samples
            let value = interpolate_at_depth(
                target_depth,
                original_depths,
                &input_curve.values,
            );
            new_values.push(value);
        }

        let output_curve = OutputCurveData {
            mnemonic: format!("{}_RS", input_curve.mnemonic),
            curve_type: input_curve.curve_type,
            unit: input_curve.unit.clone(),
            depths: new_depths.clone(),
            values: new_values,
            description: Some(format!(
                "Resampled {} at {:.3} step from {:.2} to {:.2}",
                input_curve.mnemonic, new_step, start_depth, end_depth
            )),
        };

        let mut output = UdfOutput::new(output_curve);
        output.add_metadata("new_step", serde_json::json!(new_step));
        output.add_metadata("start_depth", serde_json::json!(start_depth));
        output.add_metadata("end_depth", serde_json::json!(end_depth));
        output.add_metadata("sample_count", serde_json::json!(new_depths.len()));

        Ok(output)
    }
}

/// Linear interpolation at a target depth.
fn interpolate_at_depth(
    target: f64,
    depths: &[f64],
    values: &[Option<f64>],
) -> Option<f64> {
    if depths.is_empty() {
        return None;
    }

    // Check if outside range
    if target < depths[0] || target > depths[depths.len() - 1] {
        return None;
    }

    // Find bracketing indices using binary search
    let idx = depths.partition_point(|&d| d < target);

    if idx == 0 {
        // Exactly at or before first point
        return values[0];
    }

    if idx >= depths.len() {
        // At or after last point
        return values[depths.len() - 1];
    }

    // Check for exact match
    if (depths[idx] - target).abs() < 1e-10 {
        return values[idx];
    }

    // Linear interpolation between idx-1 and idx
    let d0 = depths[idx - 1];
    let d1 = depths[idx];
    let v0 = values[idx - 1];
    let v1 = values[idx];

    match (v0, v1) {
        (Some(val0), Some(val1)) => {
            let t = (target - d0) / (d1 - d0);
            Some(val0 + t * (val1 - val0))
        }
        (Some(val), None) | (None, Some(val)) => Some(val), // Use available value
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::parameters::ParameterValues;
    use crate::compute::types::CurveData;
    use std::collections::HashMap;

    fn create_test_curve() -> Arc<CurveData> {
        Arc::new(CurveData {
            curve_id: uuid::Uuid::new_v4(),
            mnemonic: "TEST".to_string(),
            curve_type: CurveDataType::Unknown,
            unit: "units".to_string(),
            depths: Arc::new(vec![100.0, 100.5, 101.0, 101.5, 102.0]),
            values: vec![
                Some(10.0),
                Some(20.0),
                Some(30.0),
                Some(40.0),
                Some(50.0),
            ],
            parquet_hash: "test_hash".to_string(),
            version: 1,
        })
    }

    #[test]
    fn test_moving_average() {
        let udf = MovingAverageUdf::new();
        let curve = create_test_curve();

        let mut params = HashMap::new();
        params.insert(
            "window_size".to_string(),
            crate::compute::ParameterValue::Number(3.0),
        );

        let mut context = crate::compute::context::ExecutionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            ParameterValues::from_map(params),
        );
        context.add_curve("input_curve".to_string(), curve);

        let result = udf.execute(&context).unwrap();
        assert_eq!(result.curve_data.values.len(), 5);

        // Middle value should be average of 20, 30, 40 = 30
        assert!((result.curve_data.values[2].unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_linear_scale() {
        let udf = LinearScaleUdf::new();
        let curve = create_test_curve();

        let mut params = HashMap::new();
        params.insert("in_min".to_string(), crate::compute::ParameterValue::Number(10.0));
        params.insert("in_max".to_string(), crate::compute::ParameterValue::Number(50.0));
        params.insert("out_min".to_string(), crate::compute::ParameterValue::Number(0.0));
        params.insert("out_max".to_string(), crate::compute::ParameterValue::Number(1.0));

        let mut context = crate::compute::context::ExecutionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            ParameterValues::from_map(params),
        );
        context.add_curve("input_curve".to_string(), curve);

        let result = udf.execute(&context).unwrap();

        // 10 -> 0, 50 -> 1
        assert!((result.curve_data.values[0].unwrap() - 0.0).abs() < 0.01);
        assert!((result.curve_data.values[4].unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_provider_loads_all_udfs() {
        let provider = CoreProvider::new();
        let udfs = provider.load_udfs();

        assert_eq!(udfs.len(), 3);

        let ids: Vec<_> = udfs.iter().map(|u| u.id()).collect();
        assert!(ids.contains(&"moving_average"));
        assert!(ids.contains(&"linear_scale"));
        assert!(ids.contains(&"depth_resample"));
    }
}
