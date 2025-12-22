//! Core types for the UDF system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Curve data type classification.
///
/// This enum mirrors the MainCurveType from las-types but is specific
/// to the compute engine. UDFs declare which curve types they accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurveDataType {
    /// Gamma Ray - natural radioactivity measurement
    GammaRay,
    /// Bulk Density - formation density measurement
    Density,
    /// Neutron Porosity - hydrogen index measurement
    NeutronPorosity,
    /// Resistivity - electrical resistance measurement
    Resistivity,
    /// Caliper - borehole diameter measurement
    Caliper,
    /// Sonic - acoustic travel time measurement
    Sonic,
    /// Spontaneous Potential - natural electrical potential
    SpontaneousPotential,
    /// Photo-electric Factor - lithology indicator
    PhotoelectricFactor,
    /// Depth index curve
    Depth,
    /// Computed/derived curve (output from UDFs)
    Computed,
    /// Unknown or unmapped curve type
    Unknown,
}

impl CurveDataType {
    /// Get the display name for this curve type
    pub fn display_name(&self) -> &'static str {
        match self {
            CurveDataType::GammaRay => "Gamma Ray",
            CurveDataType::Density => "Bulk Density",
            CurveDataType::NeutronPorosity => "Neutron Porosity",
            CurveDataType::Resistivity => "Resistivity",
            CurveDataType::Caliper => "Caliper",
            CurveDataType::Sonic => "Sonic",
            CurveDataType::SpontaneousPotential => "Spontaneous Potential",
            CurveDataType::PhotoelectricFactor => "Photo-electric Factor",
            CurveDataType::Depth => "Depth",
            CurveDataType::Computed => "Computed",
            CurveDataType::Unknown => "Unknown",
        }
    }

    /// Get standard units for this curve type
    pub fn standard_unit(&self) -> &'static str {
        match self {
            CurveDataType::GammaRay => "gAPI",
            CurveDataType::Density => "g/cm³",
            CurveDataType::NeutronPorosity => "v/v",
            CurveDataType::Resistivity => "ohm-m",
            CurveDataType::Caliper => "in",
            CurveDataType::Sonic => "μs/ft",
            CurveDataType::SpontaneousPotential => "mV",
            CurveDataType::PhotoelectricFactor => "b/e",
            CurveDataType::Depth => "m",
            CurveDataType::Computed => "",
            CurveDataType::Unknown => "",
        }
    }

    /// Convert from MainCurveType string representation
    pub fn from_main_curve_type(mct: &str) -> Self {
        match mct.to_uppercase().as_str() {
            "GR" => CurveDataType::GammaRay,
            "RHOB" => CurveDataType::Density,
            "NPHI" => CurveDataType::NeutronPorosity,
            "RT" => CurveDataType::Resistivity,
            "CALI" => CurveDataType::Caliper,
            "DT" => CurveDataType::Sonic,
            "SP" => CurveDataType::SpontaneousPotential,
            "PE" => CurveDataType::PhotoelectricFactor,
            "DEPTH" => CurveDataType::Depth,
            _ => CurveDataType::Unknown,
        }
    }
}

/// Immutable curve data for UDF inputs.
///
/// Curve data is loaded once and shared via Arc for efficiency.
/// UDFs receive read-only access to input data.
#[derive(Debug, Clone)]
pub struct CurveData {
    /// Curve identifier from DataForge
    pub curve_id: Uuid,
    /// Original mnemonic name
    pub mnemonic: String,
    /// Detected curve type
    pub curve_type: CurveDataType,
    /// Unit of measurement
    pub unit: String,
    /// Depth values (shared across curves from same well)
    pub depths: Arc<Vec<f64>>,
    /// Measurement values (None = null/missing)
    pub values: Vec<Option<f64>>,
    /// Content hash for provenance tracking
    pub parquet_hash: String,
    /// Curve version for optimistic concurrency
    pub version: i64,
}

impl CurveData {
    /// Get the number of samples in this curve
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if curve has no data
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get depth range as (min, max)
    pub fn depth_range(&self) -> Option<(f64, f64)> {
        if self.depths.is_empty() {
            return None;
        }
        let min = self.depths.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = self.depths.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        Some((min, max))
    }

    /// Get value at a specific index
    pub fn value_at(&self, index: usize) -> Option<f64> {
        self.values.get(index).copied().flatten()
    }

    /// Iterate over (depth, value) pairs
    pub fn iter(&self) -> impl Iterator<Item = (f64, Option<f64>)> + '_ {
        self.depths.iter().copied().zip(self.values.iter().copied())
    }

    /// Get non-null values with their depths
    pub fn valid_values(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.iter().filter_map(|(d, v)| v.map(|val| (d, val)))
    }
}

/// UDF metadata for display and documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfMetadata {
    /// Human-readable name
    pub name: String,
    /// Category for grouping (e.g., "Petrophysics", "Quality Control")
    pub category: String,
    /// Short description
    pub description: String,
    /// Detailed documentation (markdown supported)
    pub documentation: Option<String>,
    /// Version string
    pub version: String,
    /// Tags for search/filtering
    pub tags: Vec<String>,
}

/// Reference to an input curve used in execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputReference {
    /// Curve UUID
    pub curve_id: Uuid,
    /// Curve version at time of execution
    pub version: i64,
    /// Parquet content hash
    pub parquet_hash: String,
}

/// Status of a UDF execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Completed,
    /// Execution failed with error
    Failed,
    /// Execution was cancelled by user
    Cancelled,
}

/// Record of a UDF execution for provenance tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique execution ID
    pub id: Uuid,
    /// Full UDF identifier (provider:udf_id)
    pub udf_id: String,
    /// UDF version at time of execution
    pub udf_version: String,
    /// Input curve references
    pub inputs: Vec<InputReference>,
    /// Parameter values used
    pub parameters: serde_json::Value,
    /// Output curve ID (if successful)
    pub output_curve_id: Option<Uuid>,
    /// Output parquet content hash
    pub output_parquet_hash: Option<String>,
    /// Execution start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Execution completion time
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Compute application version
    pub compute_app_version: String,
    /// Execution status
    pub status: ExecutionStatus,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Output from a UDF execution.
#[derive(Debug, Clone)]
pub struct UdfOutput {
    /// Output curve data
    pub curve_data: OutputCurveData,
    /// Optional metadata to attach to output
    pub metadata: HashMap<String, serde_json::Value>,
    /// Warnings generated during execution
    pub warnings: Vec<String>,
}

/// Output curve data from a UDF.
#[derive(Debug, Clone)]
pub struct OutputCurveData {
    /// Suggested mnemonic for the output curve
    pub mnemonic: String,
    /// Output curve type
    pub curve_type: CurveDataType,
    /// Unit of measurement
    pub unit: String,
    /// Depth values
    pub depths: Vec<f64>,
    /// Computed values
    pub values: Vec<Option<f64>>,
    /// Description of the output
    pub description: Option<String>,
}

impl UdfOutput {
    /// Create a new UDF output with curve data
    pub fn new(curve_data: OutputCurveData) -> Self {
        Self {
            curve_data,
            metadata: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a warning message
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }
}
