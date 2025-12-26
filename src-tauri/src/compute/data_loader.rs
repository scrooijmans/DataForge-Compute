//! Data loader implementation for accessing DataForge curve data.
//!
//! This module provides the bridge between the UDF execution engine
//! and the DataForge database/blob storage.

use crate::compute::engine::{CurveLoader, CurveMetadataInfo};
use crate::compute::error::UdfError;
use crate::compute::types::{CurveData, CurveDataType};
use duckdb::Connection as DuckDbConnection;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// DataForge curve loader that reads from SQLite metadata and Parquet blobs.
pub struct DataForgeCurveLoader<'a> {
    /// Reference to the SQLite database connection
    db: &'a Connection,
    /// Path to the blobs directory
    blobs_dir: PathBuf,
    /// Cached depth arrays by well_id to share across curves
    depth_cache: std::cell::RefCell<std::collections::HashMap<Uuid, Arc<Vec<f64>>>>,
}

impl<'a> DataForgeCurveLoader<'a> {
    /// Create a new curve loader.
    pub fn new(db: &'a Connection, blobs_dir: PathBuf) -> Self {
        Self {
            db,
            blobs_dir,
            depth_cache: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    /// Get the path to a parquet blob by its hash.
    fn blob_path(&self, hash: &str) -> PathBuf {
        self.blobs_dir
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(format!("{}.parquet", hash))
    }

    /// Map a mnemonic to a curve type.
    fn detect_curve_type(&self, mnemonic: &str, main_curve_type: Option<&str>) -> CurveDataType {
        // First check if we have a stored main_curve_type
        if let Some(mct) = main_curve_type {
            return CurveDataType::from_main_curve_type(mct);
        }

        // Fallback: detect from mnemonic
        let upper = mnemonic.to_uppercase();
        if upper.contains("GR") || upper.contains("GAMMA") {
            CurveDataType::GammaRay
        } else if upper.contains("RHOB") || upper.contains("DENSITY") {
            CurveDataType::Density
        } else if upper.contains("NPHI") || upper.contains("NEUTRON") {
            CurveDataType::NeutronPorosity
        } else if upper.contains("RT") || upper.contains("RES") || upper.contains("ILD") {
            CurveDataType::Resistivity
        } else if upper.contains("CALI") || upper.contains("CALIPER") {
            CurveDataType::Caliper
        } else if upper.contains("DT") || upper.contains("SONIC") {
            CurveDataType::Sonic
        } else if upper.contains("SP") {
            CurveDataType::SpontaneousPotential
        } else if upper.contains("PE") || upper.contains("PHOTO") {
            CurveDataType::PhotoelectricFactor
        } else if upper.contains("DEPTH") {
            CurveDataType::Depth
        } else if upper.contains("VSH") || upper.contains("PHI") || upper.contains("SW") {
            CurveDataType::Computed
        } else {
            CurveDataType::Unknown
        }
    }
}

impl<'a> CurveLoader for DataForgeCurveLoader<'a> {
    fn load_curve(&self, curve_id: Uuid) -> Result<Arc<CurveData>, UdfError> {
        // Query curve metadata with join to curve_properties
        // DataForge uses property_id -> curve_properties.id for curve type
        let (mnemonic, unit, parquet_hash, version, well_id, property_id): (
            String,
            Option<String>,
            Option<String>,
            i64,
            String,
            Option<String>,
        ) = self
            .db
            .query_row(
                r#"SELECT c.mnemonic, c.unit,
                          COALESCE(c.gridded_parquet_hash, c.native_parquet_hash),
                          c.version, c.well_id, cp.id as property_id
                   FROM curves c
                   LEFT JOIN curve_properties cp ON c.property_id = cp.id
                   WHERE c.id = ?1"#,
                [curve_id.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get::<_, i64>(3).unwrap_or(1),
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .map_err(|e| UdfError::CurveLoadError(format!("Curve not found: {}", e)))?;

        // Convert property_id to MainCurveType format
        let main_curve_type = property_id.map(|pid| property_id_to_curve_type_code(&pid));

        let parquet_hash =
            parquet_hash.ok_or_else(|| UdfError::CurveLoadError("Curve has no data".to_string()))?;

        let blob_path = self.blob_path(&parquet_hash);

        if !blob_path.exists() {
            return Err(UdfError::CurveLoadError(format!(
                "Parquet blob not found at {:?}",
                blob_path
            )));
        }

        // Read parquet data with DuckDB
        let duckdb = DuckDbConnection::open_in_memory()
            .map_err(|e| UdfError::CurveLoadError(format!("DuckDB error: {}", e)))?;

        // DataForge stores parquet with schema:
        // - Native: [DEPTH: f64, {mnemonic}: f64]
        // - Gridded: [DEPTH_INDEX: i64, {mnemonic}: f64]
        // We need to handle both cases and use the mnemonic as the value column name
        let escaped_path = blob_path.to_string_lossy().replace('\'', "''");
        let escaped_mnemonic = mnemonic.replace('"', "\"\"");

        // First, query the parquet schema to determine which depth column exists
        let schema_query = format!(
            "SELECT column_name FROM parquet_schema('{}') WHERE column_name IN ('DEPTH', 'DEPTH_INDEX')",
            escaped_path
        );

        let depth_column: String = duckdb
            .query_row(&schema_query, [], |row| row.get(0))
            .unwrap_or_else(|_| "DEPTH".to_string()); // Default to DEPTH if query fails

        // Query with the correct depth column and mnemonic as value column
        let query = format!(
            r#"SELECT "{}" as depth, "{}" as value
            FROM read_parquet('{}')
            ORDER BY depth"#,
            depth_column,
            escaped_mnemonic,
            escaped_path
        );

        let mut stmt = duckdb
            .prepare(&query)
            .map_err(|e| UdfError::CurveLoadError(format!("Query error: {}", e)))?;

        let mut depths: Vec<f64> = Vec::new();
        let mut values: Vec<Option<f64>> = Vec::new();

        let mut rows = stmt
            .query([])
            .map_err(|e| UdfError::CurveLoadError(format!("Query error: {}", e)))?;

        while let Some(row) = rows
            .next()
            .map_err(|e| UdfError::CurveLoadError(format!("Row error: {}", e)))?
        {
            let depth: f64 = row.get(0).unwrap_or(0.0);
            let value: Option<f64> = row.get(1).ok();
            depths.push(depth);
            values.push(value);
        }

        // Try to share depth array with other curves from same well
        let well_uuid = Uuid::parse_str(&well_id)
            .map_err(|e| UdfError::CurveLoadError(format!("Invalid well UUID: {}", e)))?;

        let depths_arc = {
            let mut cache = self.depth_cache.borrow_mut();
            if let Some(cached) = cache.get(&well_uuid) {
                // Verify depths match (they should for same well)
                if cached.len() == depths.len() {
                    cached.clone()
                } else {
                    let new_arc = Arc::new(depths);
                    cache.insert(well_uuid, new_arc.clone());
                    new_arc
                }
            } else {
                let new_arc = Arc::new(depths);
                cache.insert(well_uuid, new_arc.clone());
                new_arc
            }
        };

        let curve_type = self.detect_curve_type(&mnemonic, main_curve_type.as_deref());

        Ok(Arc::new(CurveData {
            curve_id,
            mnemonic,
            curve_type,
            unit: unit.unwrap_or_default(),
            depths: depths_arc,
            values,
            parquet_hash,
            version,
        }))
    }

    fn load_curve_metadata(&self, curve_id: Uuid) -> Result<CurveMetadataInfo, UdfError> {
        let (mnemonic, unit, row_count, property_id): (String, Option<String>, i64, Option<String>) = self
            .db
            .query_row(
                r#"SELECT c.mnemonic, c.unit,
                          COALESCE(c.native_sample_count, 0),
                          cp.id as property_id
                   FROM curves c
                   LEFT JOIN curve_properties cp ON c.property_id = cp.id
                   WHERE c.id = ?1"#,
                [curve_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|e| UdfError::CurveLoadError(format!("Curve not found: {}", e)))?;

        let main_curve_type = property_id.map(|pid| property_id_to_curve_type_code(&pid));
        let curve_type = self.detect_curve_type(&mnemonic, main_curve_type.as_deref());

        Ok(CurveMetadataInfo {
            curve_id,
            mnemonic,
            curve_type,
            unit: unit.unwrap_or_default(),
            row_count,
        })
    }
}

/// Convert DataForge property_id to MainCurveType code
fn property_id_to_curve_type_code(property_id: &str) -> String {
    match property_id {
        "gamma_ray" => "GR".to_string(),
        "bulk_density" => "RHOB".to_string(),
        "neutron_porosity" => "NPHI".to_string(),
        "deep_resistivity" | "medium_resistivity" | "shallow_resistivity" => "RT".to_string(),
        "caliper" => "CALI".to_string(),
        "compressional_slowness" | "shear_slowness" => "DT".to_string(),
        "spontaneous_potential" => "SP".to_string(),
        "photoelectric" => "PE".to_string(),
        "depth" => "DEPTH".to_string(),
        _ => "OTHER".to_string(),
    }
}

/// Schema for storing execution records.
pub const EXECUTION_RECORDS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS execution_records (
    id TEXT PRIMARY KEY,
    udf_id TEXT NOT NULL,
    udf_version TEXT NOT NULL,
    inputs TEXT NOT NULL,           -- JSON array of InputReference
    parameters TEXT NOT NULL,       -- JSON object of parameter values
    output_curve_id TEXT,
    output_parquet_hash TEXT,
    started_at TEXT NOT NULL,       -- ISO 8601 timestamp
    completed_at TEXT,              -- ISO 8601 timestamp
    compute_app_version TEXT NOT NULL,
    status TEXT NOT NULL,           -- 'completed', 'failed', 'cancelled'
    error_message TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_execution_records_udf ON execution_records(udf_id);
CREATE INDEX IF NOT EXISTS idx_execution_records_output ON execution_records(output_curve_id);
CREATE INDEX IF NOT EXISTS idx_execution_records_status ON execution_records(status);
"#;

/// Save an execution record to the database.
pub fn save_execution_record(
    db: &Connection,
    record: &crate::compute::types::ExecutionRecord,
) -> Result<(), UdfError> {
    let inputs_json = serde_json::to_string(&record.inputs)?;
    let params_json = record.parameters.to_string();
    let status = match record.status {
        crate::compute::types::ExecutionStatus::Completed => "completed",
        crate::compute::types::ExecutionStatus::Failed => "failed",
        crate::compute::types::ExecutionStatus::Cancelled => "cancelled",
    };

    db.execute(
        "INSERT INTO execution_records (
            id, udf_id, udf_version, inputs, parameters,
            output_curve_id, output_parquet_hash,
            started_at, completed_at, compute_app_version,
            status, error_message
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            record.id.to_string(),
            record.udf_id,
            record.udf_version,
            inputs_json,
            params_json,
            record.output_curve_id.map(|u| u.to_string()),
            record.output_parquet_hash,
            record.started_at.to_rfc3339(),
            record.completed_at.map(|t| t.to_rfc3339()),
            record.compute_app_version,
            status,
            record.error_message,
        ],
    )?;

    Ok(())
}

/// Initialize the compute database schema.
pub fn init_compute_schema(db: &Connection) -> Result<(), UdfError> {
    db.execute_batch(EXECUTION_RECORDS_SCHEMA)?;
    Ok(())
}
