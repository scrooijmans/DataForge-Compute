//! Tauri commands for DataForge Compute
//!
//! This module provides read-only access to DataForge's shared data and
//! implements computation functions that can be run on the data.

use crate::compute;
use crate::compute::data_loader::DataForgeCurveLoader;
use crate::compute::engine::ExecutionEngine;
use crate::compute::parameters::ParameterValue;
use crate::compute::providers::register_builtin_providers;
use crate::compute::registry::{ProviderInfo, UdfInfo, UdfRegistry};
use duckdb::Connection as DuckDbConnection;
use log::info;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

/// State for the Compute application
pub struct ComputeState {
    /// Path to DataForge's app data directory
    pub dataforge_data_dir: Option<PathBuf>,
    /// Read-only connection to DataForge's SQLite database
    pub db: Option<Connection>,
    /// UDF Registry
    pub registry: Option<Arc<UdfRegistry>>,
    /// Execution Engine
    pub engine: Option<ExecutionEngine>,
}

impl Default for ComputeState {
    fn default() -> Self {
        Self {
            dataforge_data_dir: None,
            db: None,
            registry: None,
            engine: None,
        }
    }
}

impl ComputeState {
    /// Initialize the compute state by locating DataForge's data directory
    pub fn initialize(&mut self) -> anyhow::Result<()> {
        // DataForge stores its data in the standard app data directory
        // On macOS: ~/Library/Application Support/com.dataforge.app/
        // On Windows: C:\Users\<user>\AppData\Roaming\com.dataforge.app\
        // On Linux: ~/.local/share/com.dataforge.app/

        let dataforge_dir = Self::find_dataforge_data_dir()?;
        let db_path = dataforge_dir.join("dataforge.db");

        if !db_path.exists() {
            return Err(anyhow::anyhow!(
                "DataForge database not found at {:?}. Please ensure DataForge is installed and has been run at least once.",
                db_path
            ));
        }

        info!("📂 Found DataForge data at: {:?}", dataforge_dir);

        // Open database in read-only mode
        let db = Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        self.dataforge_data_dir = Some(dataforge_dir);
        self.db = Some(db);

        // Initialize UDF registry with built-in providers
        let mut registry = UdfRegistry::new();
        register_builtin_providers(&mut registry)?;

        info!(
            "📦 Registered {} providers with {} UDFs",
            registry.provider_count(),
            registry.udf_count()
        );

        let registry = Arc::new(registry);

        // Create execution engine
        let engine = ExecutionEngine::new(
            registry.clone(),
            env!("CARGO_PKG_VERSION").to_string(),
        );

        self.registry = Some(registry);
        self.engine = Some(engine);

        Ok(())
    }

    /// Find DataForge's app data directory based on platform
    pub fn find_dataforge_data_dir() -> anyhow::Result<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")?;
            Ok(PathBuf::from(home)
                .join("Library/Application Support")
                .join("com.dataforge.app"))
        }

        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA")?;
            Ok(PathBuf::from(appdata).join("com.dataforge.app"))
        }

        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME")?;
            Ok(PathBuf::from(home)
                .join(".local/share")
                .join("com.dataforge.app"))
        }
    }

    /// Get the blobs directory path
    pub fn blobs_dir(&self) -> Option<PathBuf> {
        self.dataforge_data_dir.as_ref().map(|d| d.join("blobs"))
    }

    /// Get the full path to a parquet blob by its hash
    fn blob_path(&self, hash: &str) -> Option<PathBuf> {
        self.blobs_dir().map(|blobs| {
            blobs
                .join(&hash[..2])
                .join(&hash[2..4])
                .join(format!("{}.parquet", hash))
        })
    }
}

// ==== Response Types ====

#[derive(Debug, Serialize)]
pub struct DataForgeStatus {
    pub connected: bool,
    pub data_dir: Option<String>,
    pub db_exists: bool,
    pub provider_count: usize,
    pub udf_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WellInfo {
    pub id: String,
    pub name: String,
    pub uwi: Option<String>,
    pub field: Option<String>,
    pub curve_count: i64,
}

#[derive(Debug, Serialize)]
pub struct CurveInfo {
    pub id: String,
    pub mnemonic: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub main_curve_type: Option<String>,
    pub min_depth: Option<f64>,
    pub max_depth: Option<f64>,
    pub row_count: i64,
}

#[derive(Debug, Serialize)]
pub struct CurveDataPoint {
    pub depth: f64,
    pub value: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct CurveData {
    pub curve_id: String,
    pub mnemonic: String,
    pub unit: Option<String>,
    pub data: Vec<CurveDataPoint>,
}

#[derive(Debug, Serialize)]
pub struct MovingAverageResult {
    pub input_curve: String,
    pub window_size: usize,
    pub data: Vec<CurveDataPoint>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteUdfResult {
    pub success: bool,
    pub execution_id: String,
    pub output_mnemonic: Option<String>,
    pub output_data: Option<Vec<CurveDataPoint>>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteUdfRequest {
    pub udf_id: String,
    pub well_id: String,
    pub workspace_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

// ==== Tauri Commands ====

/// Get the status of the DataForge connection
#[tauri::command]
pub fn get_dataforge_status(state: State<'_, Mutex<ComputeState>>) -> DataForgeStatus {
    let state = state.lock().expect("Failed to lock state");

    if state.db.is_none() {
        // Try to provide helpful error message
        let data_dir = ComputeState::find_dataforge_data_dir();
        let db_exists = data_dir
            .as_ref()
            .map(|p| p.join("dataforge.db").exists())
            .unwrap_or(false);
        return DataForgeStatus {
            connected: false,
            data_dir: data_dir.ok().map(|p| p.to_string_lossy().to_string()),
            db_exists,
            provider_count: 0,
            udf_count: 0,
            error: Some("Not connected to DataForge database".to_string()),
        };
    }

    let (provider_count, udf_count) = state
        .registry
        .as_ref()
        .map(|r| (r.provider_count(), r.udf_count()))
        .unwrap_or((0, 0));

    DataForgeStatus {
        connected: true,
        data_dir: state
            .dataforge_data_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        db_exists: true,
        provider_count,
        udf_count,
        error: None,
    }
}

/// List all workspaces in DataForge
#[tauri::command]
pub fn list_workspaces(state: State<'_, Mutex<ComputeState>>) -> Result<Vec<WorkspaceInfo>, String> {
    let state = state.lock().expect("Failed to lock state");
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;

    let mut stmt = db
        .prepare("SELECT id, name, created_at FROM workspaces ORDER BY name")
        .map_err(|e| format!("Query error: {}", e))?;

    let workspaces = stmt
        .query_map([], |row| {
            Ok(WorkspaceInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))?;

    Ok(workspaces)
}

/// List all wells in a workspace
#[tauri::command]
pub fn list_wells(
    workspace_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<Vec<WellInfo>, String> {
    let state = state.lock().expect("Failed to lock state");
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;

    let mut stmt = db
        .prepare(
            "SELECT w.id, w.name, w.uwi, w.field,
                    (SELECT COUNT(*) FROM curves c WHERE c.well_id = w.id) as curve_count
             FROM wells w
             WHERE w.workspace_id = ?1
             ORDER BY w.name",
        )
        .map_err(|e| format!("Query error: {}", e))?;

    let wells = stmt
        .query_map([&workspace_id], |row| {
            Ok(WellInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                uwi: row.get(2)?,
                field: row.get(3)?,
                curve_count: row.get(4)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))?;

    Ok(wells)
}

/// List all curves for a well
#[tauri::command]
pub fn list_curves(
    well_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<Vec<CurveInfo>, String> {
    let state = state.lock().expect("Failed to lock state");
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;

    let mut stmt = db
        .prepare(
            "SELECT id, mnemonic, unit, description, main_curve_type, min_depth, max_depth, row_count
             FROM curves
             WHERE well_id = ?1
             ORDER BY mnemonic",
        )
        .map_err(|e| format!("Query error: {}", e))?;

    let curves = stmt
        .query_map([&well_id], |row| {
            Ok(CurveInfo {
                id: row.get(0)?,
                mnemonic: row.get(1)?,
                unit: row.get(2)?,
                description: row.get(3)?,
                main_curve_type: row.get(4)?,
                min_depth: row.get(5)?,
                max_depth: row.get(6)?,
                row_count: row.get(7)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))?;

    Ok(curves)
}

/// Get curve data by reading the parquet blob
#[tauri::command]
pub fn get_curve_data(
    curve_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<CurveData, String> {
    let state = state.lock().expect("Failed to lock state");
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;

    // Get curve metadata and parquet hash
    let (mnemonic, unit, parquet_hash): (String, Option<String>, Option<String>) = db
        .query_row(
            "SELECT mnemonic, unit, parquet_hash FROM curves WHERE id = ?1",
            [&curve_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("Curve not found: {}", e))?;

    let parquet_hash = parquet_hash.ok_or("Curve has no data (no parquet hash)")?;

    // Get blob path
    let blob_path = state
        .blob_path(&parquet_hash)
        .ok_or("DataForge data directory not set")?;

    if !blob_path.exists() {
        return Err(format!("Parquet blob not found at {:?}", blob_path));
    }

    // Read parquet with DuckDB
    let duckdb = DuckDbConnection::open_in_memory()
        .map_err(|e| format!("Failed to create DuckDB connection: {}", e))?;

    let query = format!(
        "SELECT depth, value FROM read_parquet('{}') ORDER BY depth",
        blob_path.to_string_lossy().replace('\'', "''")
    );

    let mut stmt = duckdb
        .prepare(&query)
        .map_err(|e| format!("DuckDB query error: {}", e))?;

    let data: Vec<CurveDataPoint> = stmt
        .query_map([], |row| {
            Ok(CurveDataPoint {
                depth: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("DuckDB query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(CurveData {
        curve_id,
        mnemonic,
        unit,
        data,
    })
}

/// Compute a moving average on curve data
#[tauri::command]
pub fn compute_moving_average(
    curve_id: String,
    window_size: usize,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<MovingAverageResult, String> {
    if window_size < 1 {
        return Err("Window size must be at least 1".to_string());
    }

    // Get the curve data first
    let curve_data = get_curve_data(curve_id.clone(), state)?;

    // Compute moving average
    let mut smoothed_data = Vec::with_capacity(curve_data.data.len());
    let half_window = window_size / 2;

    for (i, point) in curve_data.data.iter().enumerate() {
        // Calculate window bounds
        let start = i.saturating_sub(half_window);
        let end = (i + half_window + 1).min(curve_data.data.len());

        // Calculate average of valid values in window
        let window_values: Vec<f64> = curve_data.data[start..end]
            .iter()
            .filter_map(|p| p.value)
            .collect();

        let avg_value = if window_values.is_empty() {
            None
        } else {
            Some(window_values.iter().sum::<f64>() / window_values.len() as f64)
        };

        smoothed_data.push(CurveDataPoint {
            depth: point.depth,
            value: avg_value,
        });
    }

    Ok(MovingAverageResult {
        input_curve: curve_data.mnemonic,
        window_size,
        data: smoothed_data,
    })
}

// ==== UDF Commands ====

/// List all available UDF providers
#[tauri::command]
pub fn list_providers(state: State<'_, Mutex<ComputeState>>) -> Result<Vec<ProviderInfo>, String> {
    let state = state.lock().expect("Failed to lock state");
    let registry = state.registry.as_ref().ok_or("Registry not initialized")?;

    Ok(registry.list_providers())
}

/// List all available UDFs
#[tauri::command]
pub fn list_udfs(state: State<'_, Mutex<ComputeState>>) -> Result<Vec<UdfInfo>, String> {
    let state = state.lock().expect("Failed to lock state");
    let registry = state.registry.as_ref().ok_or("Registry not initialized")?;

    Ok(registry.list_udfs())
}

/// Get UDF parameter definitions
#[tauri::command]
pub fn get_udf_parameters(
    udf_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let state = state.lock().expect("Failed to lock state");
    let engine = state.engine.as_ref().ok_or("Engine not initialized")?;

    engine
        .get_parameter_definitions(&udf_id)
        .map_err(|e| e.to_string())
}

/// Execute a UDF
#[tauri::command]
pub fn execute_udf(
    request: ExecuteUdfRequest,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<ExecuteUdfResult, String> {
    let state = state.lock().expect("Failed to lock state");

    let engine = state.engine.as_ref().ok_or("Engine not initialized")?;
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;
    let blobs_dir = state.blobs_dir().ok_or("Blobs directory not set")?;

    // Parse UUIDs
    let well_id = Uuid::parse_str(&request.well_id)
        .map_err(|e| format!("Invalid well ID: {}", e))?;
    let workspace_id = Uuid::parse_str(&request.workspace_id)
        .map_err(|e| format!("Invalid workspace ID: {}", e))?;

    // Convert JSON parameters to ParameterValue
    let parameters: HashMap<String, ParameterValue> = request
        .parameters
        .into_iter()
        .map(|(k, v)| {
            let pv = json_to_parameter_value(v);
            (k, pv)
        })
        .collect();

    // Create curve loader
    let loader = DataForgeCurveLoader::new(db, blobs_dir);

    // Execute
    let result = engine
        .execute(&request.udf_id, well_id, workspace_id, parameters, &loader)
        .map_err(|e| e.to_string())?;

    // Build response
    if let Some(output) = result.output {
        let output_data: Vec<CurveDataPoint> = output
            .curve_data
            .depths
            .iter()
            .zip(output.curve_data.values.iter())
            .map(|(d, v)| CurveDataPoint {
                depth: *d,
                value: *v,
            })
            .collect();

        Ok(ExecuteUdfResult {
            success: true,
            execution_id: result.record.id.to_string(),
            output_mnemonic: Some(output.curve_data.mnemonic),
            output_data: Some(output_data),
            warnings: output.warnings,
            error: None,
        })
    } else {
        Ok(ExecuteUdfResult {
            success: false,
            execution_id: result.record.id.to_string(),
            output_mnemonic: None,
            output_data: None,
            warnings: Vec::new(),
            error: result.record.error_message,
        })
    }
}

/// Validate UDF parameters without executing
#[tauri::command]
pub fn validate_udf_parameters(
    udf_id: String,
    parameters: HashMap<String, serde_json::Value>,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let state = state.lock().expect("Failed to lock state");
    let engine = state.engine.as_ref().ok_or("Engine not initialized")?;

    let params: HashMap<String, ParameterValue> = parameters
        .into_iter()
        .map(|(k, v)| (k, json_to_parameter_value(v)))
        .collect();

    let errors = engine
        .validate_only(&udf_id, &params)
        .map_err(|e| e.to_string())?;

    Ok(errors
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "field": e.field,
                "message": e.message,
                "suggestion": e.suggestion
            })
        })
        .collect())
}

/// Helper to convert JSON value to ParameterValue
fn json_to_parameter_value(v: serde_json::Value) -> ParameterValue {
    match v {
        serde_json::Value::Null => ParameterValue::Null,
        serde_json::Value::Bool(b) => ParameterValue::Boolean(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ParameterValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                ParameterValue::Number(f)
            } else {
                ParameterValue::Null
            }
        }
        serde_json::Value::String(s) => {
            // Try to parse as UUID for curve references
            if let Ok(uuid) = Uuid::parse_str(&s) {
                ParameterValue::Curve(uuid)
            } else {
                ParameterValue::String(s)
            }
        }
        _ => ParameterValue::Null,
    }
}
