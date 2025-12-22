//! Parameter definitions for UDFs.
//!
//! This module implements type-safe parameter definitions that UDFs use
//! to declare their inputs. Parameters support validation, default values,
//! and constraints.

use crate::compute::error::ValidationError;
use crate::compute::types::CurveDataType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

/// Dynamic parameter value that can hold different types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    /// Curve reference by UUID
    Curve(uuid::Uuid),
    /// Numeric value (f64)
    Number(f64),
    /// Integer value
    Integer(i64),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Optional value (None)
    Null,
}

impl ParameterValue {
    /// Try to get as curve UUID
    pub fn as_curve(&self) -> Option<uuid::Uuid> {
        match self {
            ParameterValue::Curve(id) => Some(*id),
            ParameterValue::String(s) => uuid::Uuid::parse_str(s).ok(),
            _ => None,
        }
    }

    /// Try to get as f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ParameterValue::Number(n) => Some(*n),
            ParameterValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get as i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ParameterValue::Integer(i) => Some(*i),
            ParameterValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    /// Try to get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ParameterValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParameterValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, ParameterValue::Null)
    }
}

/// Base trait for parameter definitions.
pub trait ParameterDefinition: Send + Sync + Debug {
    /// Parameter name (identifier)
    fn name(&self) -> &str;

    /// Human-readable label
    fn label(&self) -> &str;

    /// Description/help text
    fn description(&self) -> &str;

    /// Whether this parameter is required
    fn is_required(&self) -> bool;

    /// Default value (if any)
    fn default_value(&self) -> Option<ParameterValue>;

    /// Validate a parameter value
    fn validate(&self, value: &ParameterValue) -> Result<(), ValidationError>;

    /// Get parameter type as string
    fn param_type(&self) -> &str;

    /// Get parameter definition as JSON for frontend
    fn to_json(&self) -> serde_json::Value;
}

/// Curve input parameter with type constraints.
///
/// This is the key mechanism for ensuring UDFs only accept
/// curves of the appropriate type.
#[derive(Debug, Clone)]
pub struct CurveParameter {
    /// Parameter name
    pub name: String,
    /// Display label
    pub label: String,
    /// Description
    pub description: String,
    /// Whether this curve is required
    pub required: bool,
    /// Allowed curve types (empty = any type allowed)
    pub allowed_types: Vec<CurveDataType>,
    /// Minimum number of data points required
    pub min_length: Option<usize>,
    /// Whether null values are allowed in the curve
    pub allow_nulls: bool,
}

impl CurveParameter {
    /// Create a new required curve parameter
    pub fn required(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: String::new(),
            required: true,
            allowed_types: Vec::new(),
            min_length: None,
            allow_nulls: true,
        }
    }

    /// Create a new optional curve parameter
    pub fn optional(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: String::new(),
            required: false,
            allowed_types: Vec::new(),
            min_length: None,
            allow_nulls: true,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Restrict to specific curve types
    pub fn with_allowed_types(mut self, types: Vec<CurveDataType>) -> Self {
        self.allowed_types = types;
        self
    }

    /// Set minimum data length requirement
    pub fn with_min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Disallow null values in the curve
    pub fn require_no_nulls(mut self) -> Self {
        self.allow_nulls = false;
        self
    }

    /// Check if a curve type is allowed
    pub fn is_type_allowed(&self, curve_type: CurveDataType) -> bool {
        if self.allowed_types.is_empty() {
            true // No restrictions
        } else {
            self.allowed_types.contains(&curve_type)
        }
    }

    /// Get allowed types as display string
    pub fn allowed_types_display(&self) -> String {
        if self.allowed_types.is_empty() {
            "Any".to_string()
        } else {
            self.allowed_types
                .iter()
                .map(|t| t.display_name())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

impl ParameterDefinition for CurveParameter {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn is_required(&self) -> bool {
        self.required
    }

    fn default_value(&self) -> Option<ParameterValue> {
        None // Curves don't have defaults
    }

    fn validate(&self, value: &ParameterValue) -> Result<(), ValidationError> {
        if value.is_null() {
            if self.required {
                return Err(ValidationError::new(&self.name, "Required curve not provided"));
            }
            return Ok(());
        }

        if value.as_curve().is_none() {
            return Err(ValidationError::new(
                &self.name,
                "Value must be a valid curve UUID",
            ));
        }

        Ok(())
    }

    fn param_type(&self) -> &str {
        "curve"
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "label": self.label,
            "description": self.description,
            "type": "curve",
            "required": self.required,
            "allowed_types": self.allowed_types.iter().map(|t| t.display_name()).collect::<Vec<_>>(),
            "min_length": self.min_length,
            "allow_nulls": self.allow_nulls,
        })
    }
}

/// Numeric parameter with optional range constraints.
#[derive(Debug, Clone)]
pub struct NumericParameter {
    /// Parameter name
    pub name: String,
    /// Display label
    pub label: String,
    /// Description
    pub description: String,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value
    pub default: Option<f64>,
    /// Minimum value (inclusive)
    pub min: Option<f64>,
    /// Maximum value (inclusive)
    pub max: Option<f64>,
    /// Unit of measurement (for display)
    pub unit: Option<String>,
}

impl NumericParameter {
    /// Create a new required numeric parameter
    pub fn required(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: String::new(),
            required: true,
            default: None,
            min: None,
            max: None,
            unit: None,
        }
    }

    /// Create a new optional numeric parameter with default
    pub fn optional(name: impl Into<String>, label: impl Into<String>, default: f64) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: String::new(),
            required: false,
            default: Some(default),
            min: None,
            max: None,
            unit: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set range constraints
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Set minimum value
    pub fn with_min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value
    pub fn with_max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Set unit for display
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

impl ParameterDefinition for NumericParameter {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn is_required(&self) -> bool {
        self.required
    }

    fn default_value(&self) -> Option<ParameterValue> {
        self.default.map(ParameterValue::Number)
    }

    fn validate(&self, value: &ParameterValue) -> Result<(), ValidationError> {
        if value.is_null() {
            if self.required && self.default.is_none() {
                return Err(ValidationError::new(&self.name, "Required parameter not provided"));
            }
            return Ok(());
        }

        let num = value.as_f64().ok_or_else(|| {
            ValidationError::new(&self.name, "Value must be a number")
        })?;

        if let Some(min) = self.min {
            if num < min {
                return Err(ValidationError::new(
                    &self.name,
                    format!("Value must be >= {}", min),
                )
                .with_suggestion(format!("Enter a value of {} or greater", min)));
            }
        }

        if let Some(max) = self.max {
            if num > max {
                return Err(ValidationError::new(
                    &self.name,
                    format!("Value must be <= {}", max),
                )
                .with_suggestion(format!("Enter a value of {} or less", max)));
            }
        }

        Ok(())
    }

    fn param_type(&self) -> &str {
        "number"
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "label": self.label,
            "description": self.description,
            "type": "number",
            "required": self.required,
            "default": self.default,
            "min": self.min,
            "max": self.max,
            "unit": self.unit,
        })
    }
}

/// Parameter collection for easy access by name.
#[derive(Debug, Clone, Default)]
pub struct ParameterValues {
    values: HashMap<String, ParameterValue>,
}

impl ParameterValues {
    /// Create from a HashMap
    pub fn from_map(values: HashMap<String, ParameterValue>) -> Self {
        Self { values }
    }

    /// Get a parameter value by name
    pub fn get(&self, name: &str) -> Option<&ParameterValue> {
        self.values.get(name)
    }

    /// Get as curve UUID
    pub fn get_curve(&self, name: &str) -> Option<uuid::Uuid> {
        self.values.get(name).and_then(|v| v.as_curve())
    }

    /// Get as f64
    pub fn get_f64(&self, name: &str) -> Option<f64> {
        self.values.get(name).and_then(|v| v.as_f64())
    }

    /// Get as f64 with default
    pub fn get_f64_or(&self, name: &str, default: f64) -> f64 {
        self.get_f64(name).unwrap_or(default)
    }

    /// Get as i64
    pub fn get_i64(&self, name: &str) -> Option<i64> {
        self.values.get(name).and_then(|v| v.as_i64())
    }

    /// Get as string
    pub fn get_string(&self, name: &str) -> Option<&str> {
        self.values.get(name).and_then(|v| v.as_str())
    }

    /// Get as bool
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.values.get(name).and_then(|v| v.as_bool())
    }

    /// Get as bool with default
    pub fn get_bool_or(&self, name: &str, default: bool) -> bool {
        self.get_bool(name).unwrap_or(default)
    }

    /// Check if parameter exists and is not null
    pub fn has(&self, name: &str) -> bool {
        self.values.get(name).map(|v| !v.is_null()).unwrap_or(false)
    }

    /// Insert a value
    pub fn insert(&mut self, name: impl Into<String>, value: ParameterValue) {
        self.values.insert(name.into(), value);
    }

    /// Convert to JSON for storage
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.values).unwrap_or(serde_json::Value::Null)
    }
}
