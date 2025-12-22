//! Petrophysics UDF provider.
//!
//! This provider contains fundamental petrophysical calculations
//! commonly used in well log analysis.

use crate::compute::context::ExecutionContext;
use crate::compute::error::UdfError;
use crate::compute::parameters::{CurveParameter, NumericParameter, ParameterDefinition};
use crate::compute::types::{CurveDataType, OutputCurveData, UdfMetadata, UdfOutput};
use crate::compute::{Udf, UdfProvider};
use std::sync::Arc;

/// Petrophysics provider with fundamental well log calculations.
pub struct PetrophysicsProvider {
    version: String,
}

impl PetrophysicsProvider {
    /// Create a new petrophysics provider.
    pub fn new() -> Self {
        Self {
            version: "0.1.0".to_string(),
        }
    }
}

impl Default for PetrophysicsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UdfProvider for PetrophysicsProvider {
    fn id(&self) -> &str {
        "petro"
    }

    fn name(&self) -> &str {
        "Petrophysics"
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        "Fundamental petrophysical calculations for well log analysis"
    }

    fn load_udfs(&self) -> Vec<Arc<dyn Udf>> {
        vec![
            Arc::new(VShaleLinearUdf::new()),
            Arc::new(VShaleClavier::new()),
            Arc::new(VShaleSteiber::new()),
        ]
    }
}

// =============================================================================
// VShale Linear UDF
// =============================================================================

/// Linear VShale calculation from Gamma Ray.
///
/// The linear VShale method is the simplest approach, calculating
/// shale volume as a linear function of the gamma ray index:
///
/// Vsh = (GR - GRmin) / (GRmax - GRmin)
///
/// Where:
/// - GR is the measured gamma ray value
/// - GRmin is the clean sand (minimum shale) gamma ray reading
/// - GRmax is the shale (maximum shale) gamma ray reading
pub struct VShaleLinearUdf;

impl VShaleLinearUdf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VShaleLinearUdf {
    fn default() -> Self {
        Self::new()
    }
}

impl Udf for VShaleLinearUdf {
    fn id(&self) -> &str {
        "vshale_linear"
    }

    fn metadata(&self) -> UdfMetadata {
        UdfMetadata {
            name: "VShale (Linear)".to_string(),
            category: "Petrophysics".to_string(),
            description: "Calculate shale volume from Gamma Ray using linear method".to_string(),
            documentation: Some(
                r#"# VShale Linear

The linear VShale method calculates shale volume as a linear function of the
gamma ray index (IGR):

```
IGR = (GR - GRmin) / (GRmax - GRmin)
Vsh = IGR
```

## Parameters

- **GR Curve**: Input gamma ray curve (must be of type Gamma Ray)
- **GR Min**: Clean sand gamma ray reading (API units)
- **GR Max**: Shale gamma ray reading (API units)

## Output

- **VSH_LIN**: Shale volume fraction (0-1, may exceed bounds if GR is outside min/max range)

## Notes

- This is the simplest VShale method and often overestimates shale in consolidated formations
- For more accurate results in older formations, consider Clavier or Steiber methods
"#
                .to_string(),
            ),
            version: "1.0.0".to_string(),
            tags: vec![
                "shale".to_string(),
                "gamma ray".to_string(),
                "vshale".to_string(),
                "linear".to_string(),
            ],
        }
    }

    fn parameter_definitions(&self) -> Vec<Box<dyn ParameterDefinition>> {
        vec![
            Box::new(
                CurveParameter::required("gr_curve", "Gamma Ray Curve")
                    .with_description("Input gamma ray log for VShale calculation")
                    .with_allowed_types(vec![CurveDataType::GammaRay]),
            ),
            Box::new(
                NumericParameter::required("gr_min", "GR Clean (Min)")
                    .with_description("Gamma ray reading in clean sand zone (API units)")
                    .with_min(0.0)
                    .with_unit("gAPI"),
            ),
            Box::new(
                NumericParameter::required("gr_max", "GR Shale (Max)")
                    .with_description("Gamma ray reading in shale zone (API units)")
                    .with_min(0.0)
                    .with_unit("gAPI"),
            ),
        ]
    }

    fn check_parameters(&self, context: &ExecutionContext) -> Result<(), Vec<crate::compute::ValidationError>> {
        let params = context.parameters();
        let mut errors = Vec::new();

        let gr_min = params.get_f64("gr_min").unwrap_or(0.0);
        let gr_max = params.get_f64("gr_max").unwrap_or(0.0);

        if gr_max <= gr_min {
            errors.push(crate::compute::ValidationError::new(
                "gr_max",
                "GR Max must be greater than GR Min",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn execute(&self, context: &ExecutionContext) -> Result<UdfOutput, UdfError> {
        // Get the GR curve
        let gr_curve = context.require_curve("gr_curve")?;

        // Get parameters
        let params = context.parameters();
        let gr_min = params.get_f64("gr_min").ok_or_else(|| {
            UdfError::ParameterValidation("gr_min is required".to_string())
        })?;
        let gr_max = params.get_f64("gr_max").ok_or_else(|| {
            UdfError::ParameterValidation("gr_max is required".to_string())
        })?;

        // Calculate VShale
        let gr_range = gr_max - gr_min;
        let mut vsh_values: Vec<Option<f64>> = Vec::with_capacity(gr_curve.len());
        let mut warnings: Vec<String> = Vec::new();
        let mut out_of_bounds_count = 0;

        for value in &gr_curve.values {
            match value {
                Some(gr) => {
                    let igr = (gr - gr_min) / gr_range;
                    let vsh = igr; // Linear: Vsh = IGR

                    // Track values outside 0-1 range
                    if vsh < 0.0 || vsh > 1.0 {
                        out_of_bounds_count += 1;
                    }

                    vsh_values.push(Some(vsh));
                }
                None => {
                    vsh_values.push(None);
                }
            }
        }

        // Add warning if many values are out of bounds
        if out_of_bounds_count > 0 {
            let pct = (out_of_bounds_count as f64 / gr_curve.len() as f64) * 100.0;
            if pct > 5.0 {
                warnings.push(format!(
                    "{:.1}% of values are outside 0-1 range. Consider adjusting GR min/max.",
                    pct
                ));
            }
        }

        // Create output
        let output_curve = OutputCurveData {
            mnemonic: "VSH_LIN".to_string(),
            curve_type: CurveDataType::Computed,
            unit: "v/v".to_string(),
            depths: gr_curve.depths.as_ref().clone(),
            values: vsh_values,
            description: Some(format!(
                "VShale (Linear) from {}, GR range: {:.1}-{:.1} gAPI",
                gr_curve.mnemonic, gr_min, gr_max
            )),
        };

        let mut output = UdfOutput::new(output_curve);
        for warning in warnings {
            output.add_warning(warning);
        }

        // Add metadata about the calculation
        output.add_metadata("method", serde_json::json!("linear"));
        output.add_metadata("gr_min", serde_json::json!(gr_min));
        output.add_metadata("gr_max", serde_json::json!(gr_max));
        output.add_metadata("input_curve", serde_json::json!(gr_curve.mnemonic));

        Ok(output)
    }
}

// =============================================================================
// VShale Clavier UDF
// =============================================================================

/// VShale calculation using Clavier equation.
///
/// The Clavier method provides a non-linear correction for older,
/// more consolidated formations:
///
/// Vsh = 1.7 - sqrt(3.38 - (IGR + 0.7)^2)
///
/// Where IGR is the gamma ray index.
pub struct VShaleClavier;

impl VShaleClavier {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VShaleClavier {
    fn default() -> Self {
        Self::new()
    }
}

impl Udf for VShaleClavier {
    fn id(&self) -> &str {
        "vshale_clavier"
    }

    fn metadata(&self) -> UdfMetadata {
        UdfMetadata {
            name: "VShale (Clavier)".to_string(),
            category: "Petrophysics".to_string(),
            description: "Calculate shale volume from Gamma Ray using Clavier equation".to_string(),
            documentation: Some(
                r#"# VShale Clavier

The Clavier method provides a non-linear correction for older,
more consolidated formations:

```
IGR = (GR - GRmin) / (GRmax - GRmin)
Vsh = 1.7 - sqrt(3.38 - (IGR + 0.7)^2)
```

## When to Use

Best for Tertiary and older consolidated rocks where the linear
method tends to overestimate shale content.
"#
                .to_string(),
            ),
            version: "1.0.0".to_string(),
            tags: vec![
                "shale".to_string(),
                "gamma ray".to_string(),
                "vshale".to_string(),
                "clavier".to_string(),
            ],
        }
    }

    fn parameter_definitions(&self) -> Vec<Box<dyn ParameterDefinition>> {
        vec![
            Box::new(
                CurveParameter::required("gr_curve", "Gamma Ray Curve")
                    .with_description("Input gamma ray log for VShale calculation")
                    .with_allowed_types(vec![CurveDataType::GammaRay]),
            ),
            Box::new(
                NumericParameter::required("gr_min", "GR Clean (Min)")
                    .with_description("Gamma ray reading in clean sand zone (API units)")
                    .with_min(0.0)
                    .with_unit("gAPI"),
            ),
            Box::new(
                NumericParameter::required("gr_max", "GR Shale (Max)")
                    .with_description("Gamma ray reading in shale zone (API units)")
                    .with_min(0.0)
                    .with_unit("gAPI"),
            ),
        ]
    }

    fn check_parameters(&self, context: &ExecutionContext) -> Result<(), Vec<crate::compute::ValidationError>> {
        let params = context.parameters();
        let mut errors = Vec::new();

        let gr_min = params.get_f64("gr_min").unwrap_or(0.0);
        let gr_max = params.get_f64("gr_max").unwrap_or(0.0);

        if gr_max <= gr_min {
            errors.push(crate::compute::ValidationError::new(
                "gr_max",
                "GR Max must be greater than GR Min",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn execute(&self, context: &ExecutionContext) -> Result<UdfOutput, UdfError> {
        let gr_curve = context.require_curve("gr_curve")?;
        let params = context.parameters();
        let gr_min = params.get_f64("gr_min").unwrap();
        let gr_max = params.get_f64("gr_max").unwrap();

        let gr_range = gr_max - gr_min;
        let mut vsh_values: Vec<Option<f64>> = Vec::with_capacity(gr_curve.len());

        for value in &gr_curve.values {
            match value {
                Some(gr) => {
                    let igr = (gr - gr_min) / gr_range;
                    // Clavier equation: Vsh = 1.7 - sqrt(3.38 - (IGR + 0.7)^2)
                    let inner = 3.38 - (igr + 0.7).powi(2);
                    let vsh = if inner >= 0.0 {
                        (1.7 - inner.sqrt()).clamp(0.0, 1.0)
                    } else {
                        1.0 // Fallback for extreme values
                    };
                    vsh_values.push(Some(vsh));
                }
                None => {
                    vsh_values.push(None);
                }
            }
        }

        let output_curve = OutputCurveData {
            mnemonic: "VSH_CLAV".to_string(),
            curve_type: CurveDataType::Computed,
            unit: "v/v".to_string(),
            depths: gr_curve.depths.as_ref().clone(),
            values: vsh_values,
            description: Some(format!(
                "VShale (Clavier) from {}, GR range: {:.1}-{:.1} gAPI",
                gr_curve.mnemonic, gr_min, gr_max
            )),
        };

        let mut output = UdfOutput::new(output_curve);
        output.add_metadata("method", serde_json::json!("clavier"));
        output.add_metadata("gr_min", serde_json::json!(gr_min));
        output.add_metadata("gr_max", serde_json::json!(gr_max));

        Ok(output)
    }
}

// =============================================================================
// VShale Steiber UDF
// =============================================================================

/// VShale calculation using Steiber equation.
///
/// The Steiber method provides another non-linear correction:
///
/// Vsh = IGR / (3 - 2 * IGR)
///
/// Where IGR is the gamma ray index.
pub struct VShaleSteiber;

impl VShaleSteiber {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VShaleSteiber {
    fn default() -> Self {
        Self::new()
    }
}

impl Udf for VShaleSteiber {
    fn id(&self) -> &str {
        "vshale_steiber"
    }

    fn metadata(&self) -> UdfMetadata {
        UdfMetadata {
            name: "VShale (Steiber)".to_string(),
            category: "Petrophysics".to_string(),
            description: "Calculate shale volume from Gamma Ray using Steiber equation".to_string(),
            documentation: Some(
                r#"# VShale Steiber

The Steiber method provides a non-linear correction:

```
IGR = (GR - GRmin) / (GRmax - GRmin)
Vsh = IGR / (3 - 2 * IGR)
```

## When to Use

An alternative to Clavier for consolidated formations. May give
lower shale volume estimates than the linear method.
"#
                .to_string(),
            ),
            version: "1.0.0".to_string(),
            tags: vec![
                "shale".to_string(),
                "gamma ray".to_string(),
                "vshale".to_string(),
                "steiber".to_string(),
            ],
        }
    }

    fn parameter_definitions(&self) -> Vec<Box<dyn ParameterDefinition>> {
        vec![
            Box::new(
                CurveParameter::required("gr_curve", "Gamma Ray Curve")
                    .with_description("Input gamma ray log for VShale calculation")
                    .with_allowed_types(vec![CurveDataType::GammaRay]),
            ),
            Box::new(
                NumericParameter::required("gr_min", "GR Clean (Min)")
                    .with_description("Gamma ray reading in clean sand zone (API units)")
                    .with_min(0.0)
                    .with_unit("gAPI"),
            ),
            Box::new(
                NumericParameter::required("gr_max", "GR Shale (Max)")
                    .with_description("Gamma ray reading in shale zone (API units)")
                    .with_min(0.0)
                    .with_unit("gAPI"),
            ),
        ]
    }

    fn check_parameters(&self, context: &ExecutionContext) -> Result<(), Vec<crate::compute::ValidationError>> {
        let params = context.parameters();
        let mut errors = Vec::new();

        let gr_min = params.get_f64("gr_min").unwrap_or(0.0);
        let gr_max = params.get_f64("gr_max").unwrap_or(0.0);

        if gr_max <= gr_min {
            errors.push(crate::compute::ValidationError::new(
                "gr_max",
                "GR Max must be greater than GR Min",
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn execute(&self, context: &ExecutionContext) -> Result<UdfOutput, UdfError> {
        let gr_curve = context.require_curve("gr_curve")?;
        let params = context.parameters();
        let gr_min = params.get_f64("gr_min").unwrap();
        let gr_max = params.get_f64("gr_max").unwrap();

        let gr_range = gr_max - gr_min;
        let mut vsh_values: Vec<Option<f64>> = Vec::with_capacity(gr_curve.len());

        for value in &gr_curve.values {
            match value {
                Some(gr) => {
                    let igr = ((gr - gr_min) / gr_range).clamp(0.0, 1.0);
                    // Steiber equation: Vsh = IGR / (3 - 2 * IGR)
                    let denominator = 3.0 - 2.0 * igr;
                    let vsh = if denominator > 0.0 {
                        (igr / denominator).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    vsh_values.push(Some(vsh));
                }
                None => {
                    vsh_values.push(None);
                }
            }
        }

        let output_curve = OutputCurveData {
            mnemonic: "VSH_STEI".to_string(),
            curve_type: CurveDataType::Computed,
            unit: "v/v".to_string(),
            depths: gr_curve.depths.as_ref().clone(),
            values: vsh_values,
            description: Some(format!(
                "VShale (Steiber) from {}, GR range: {:.1}-{:.1} gAPI",
                gr_curve.mnemonic, gr_min, gr_max
            )),
        };

        let mut output = UdfOutput::new(output_curve);
        output.add_metadata("method", serde_json::json!("steiber"));
        output.add_metadata("gr_min", serde_json::json!(gr_min));
        output.add_metadata("gr_max", serde_json::json!(gr_max));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::types::CurveData;
    use crate::compute::parameters::ParameterValues;
    use std::collections::HashMap;

    fn create_test_gr_curve() -> Arc<CurveData> {
        Arc::new(CurveData {
            curve_id: uuid::Uuid::new_v4(),
            mnemonic: "GR".to_string(),
            curve_type: CurveDataType::GammaRay,
            unit: "gAPI".to_string(),
            depths: Arc::new(vec![100.0, 100.5, 101.0, 101.5, 102.0]),
            values: vec![
                Some(30.0),  // Clean
                Some(50.0),  // Mixed
                Some(70.0),  // Mixed
                Some(90.0),  // Near shale
                Some(100.0), // Shale
            ],
            parquet_hash: "test_hash".to_string(),
            version: 1,
        })
    }

    #[test]
    fn test_vshale_linear_calculation() {
        let udf = VShaleLinearUdf::new();
        let gr_curve = create_test_gr_curve();

        let mut params = HashMap::new();
        params.insert("gr_min".to_string(), crate::compute::ParameterValue::Number(30.0));
        params.insert("gr_max".to_string(), crate::compute::ParameterValue::Number(100.0));

        let mut context = crate::compute::context::ExecutionContext::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            ParameterValues::from_map(params),
        );
        context.add_curve("gr_curve".to_string(), gr_curve);

        let result = udf.execute(&context).unwrap();

        // Check output values
        let values = &result.curve_data.values;
        assert_eq!(values.len(), 5);

        // GR=30 -> Vsh=0.0
        assert!((values[0].unwrap() - 0.0).abs() < 0.01);
        // GR=100 -> Vsh=1.0
        assert!((values[4].unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_vshale_requires_gr_curve_type() {
        let udf = VShaleLinearUdf::new();
        let params = udf.parameter_definitions();

        // Find the gr_curve parameter
        let gr_param = params.iter().find(|p| p.name() == "gr_curve").unwrap();
        let json = gr_param.to_json();

        // Check that only GammaRay type is allowed
        let allowed = json["allowed_types"].as_array().unwrap();
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].as_str().unwrap(), "Gamma Ray");
    }

    #[test]
    fn test_provider_loads_all_udfs() {
        let provider = PetrophysicsProvider::new();
        let udfs = provider.load_udfs();

        assert_eq!(udfs.len(), 3);

        let ids: Vec<_> = udfs.iter().map(|u| u.id()).collect();
        assert!(ids.contains(&"vshale_linear"));
        assert!(ids.contains(&"vshale_clavier"));
        assert!(ids.contains(&"vshale_steiber"));
    }
}
