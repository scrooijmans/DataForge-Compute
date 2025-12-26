//! Tauri commands for DataForge Compute
//!
//! This module provides read-only access to DataForge's shared data and
//! implements computation functions that can be run on the data.

use crate::compute::context::{CancellationToken, ProgressState};
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
use std::sync::{Arc, Mutex, RwLock};
use tauri::State;
use uuid::Uuid;

/// Active execution tracking for progress and cancellation.
#[derive(Default)]
pub struct ActiveExecutions {
    /// Map of execution ID to cancellation token and progress state
    executions: RwLock<HashMap<String, (Arc<CancellationToken>, Arc<ProgressState>)>>,
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub output_curve_id: Option<String>,
    pub output_data: Option<Vec<CurveDataPoint>>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub saved: bool,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteUdfRequest {
    pub udf_id: String,
    pub well_id: String,
    pub workspace_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub save_result: bool,
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

    // Join with curve_properties to get the canonical curve type
    // The property_id maps to curve_properties.id (e.g., 'gamma_ray', 'bulk_density')
    // We convert this to the MainCurveType format expected by UDFs (e.g., 'GR', 'RHOB')
    let mut stmt = db
        .prepare(
            r#"SELECT c.id, c.mnemonic, c.unit, c.description,
                      cp.id as property_id,
                      COALESCE(c.min_value, c.native_top_depth) as min_depth,
                      COALESCE(c.max_value, c.native_bottom_depth) as max_depth,
                      COALESCE(c.native_sample_count, 0) as row_count
               FROM curves c
               LEFT JOIN curve_properties cp ON c.property_id = cp.id
               WHERE c.well_id = ?1 AND c.deleted_at IS NULL
               ORDER BY c.mnemonic"#,
        )
        .map_err(|e| format!("Query error: {}", e))?;

    let curves = stmt
        .query_map([&well_id], |row| {
            let property_id: Option<String> = row.get(4)?;
            // Map property_id to MainCurveType code
            let main_curve_type = property_id.map(|pid| property_id_to_curve_type(&pid));

            Ok(CurveInfo {
                id: row.get(0)?,
                mnemonic: row.get(1)?,
                unit: row.get(2)?,
                description: row.get(3)?,
                main_curve_type,
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

/// Curve info with associated well data - for curve selector dialogs
#[derive(Debug, Serialize)]
pub struct CurveInfoWithWell {
    pub id: String,
    pub mnemonic: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub main_curve_type: Option<String>,
    pub min_depth: Option<f64>,
    pub max_depth: Option<f64>,
    pub row_count: i64,
    pub well_id: String,
    pub well_name: String,
}

/// List all curves for a workspace with well information
#[tauri::command]
pub fn list_all_curves_for_workspace(
    workspace_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<Vec<CurveInfoWithWell>, String> {
    let state = state.lock().expect("Failed to lock state");
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;

    // Join curves with wells and curve_properties to get all curve info
    let mut stmt = db
        .prepare(
            r#"SELECT c.id, c.mnemonic, c.unit, c.description,
                      cp.id as property_id,
                      COALESCE(c.min_value, c.native_top_depth) as min_depth,
                      COALESCE(c.max_value, c.native_bottom_depth) as max_depth,
                      COALESCE(c.native_sample_count, 0) as row_count,
                      w.id as well_id, w.name as well_name
               FROM curves c
               JOIN wells w ON c.well_id = w.id
               LEFT JOIN curve_properties cp ON c.property_id = cp.id
               WHERE w.workspace_id = ?1 AND c.deleted_at IS NULL
               ORDER BY w.name, c.mnemonic"#,
        )
        .map_err(|e| format!("Query error: {}", e))?;

    let curves = stmt
        .query_map([&workspace_id], |row| {
            let property_id: Option<String> = row.get(4)?;
            let main_curve_type = property_id.map(|pid| property_id_to_curve_type(&pid));

            Ok(CurveInfoWithWell {
                id: row.get(0)?,
                mnemonic: row.get(1)?,
                unit: row.get(2)?,
                description: row.get(3)?,
                main_curve_type,
                min_depth: row.get(5)?,
                max_depth: row.get(6)?,
                row_count: row.get(7)?,
                well_id: row.get(8)?,
                well_name: row.get(9)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))?;

    Ok(curves)
}

/// Convert DataForge property_id to MainCurveType code
fn property_id_to_curve_type(property_id: &str) -> String {
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

/// Get curve data by reading the parquet blob
#[tauri::command]
pub fn get_curve_data(
    curve_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<CurveData, String> {
    let state = state.lock().expect("Failed to lock state");
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;

    // Get curve metadata and parquet hash
    // Prefer gridded data (resampled to well grid), fall back to native data
    let (mnemonic, unit, parquet_hash): (String, Option<String>, Option<String>) = db
        .query_row(
            "SELECT mnemonic, unit, COALESCE(gridded_parquet_hash, native_parquet_hash) FROM curves WHERE id = ?1",
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
    active_executions: State<'_, ActiveExecutions>,
) -> Result<ExecuteUdfResult, String> {
    // Generate execution ID upfront for tracking
    let execution_id = Uuid::new_v4().to_string();

    // Create cancellation token and progress state for this execution
    let cancel_token = Arc::new(CancellationToken::new());
    let progress_state = Arc::new(ProgressState::new());

    // Register this execution for progress tracking
    {
        let mut executions = active_executions
            .executions
            .write()
            .map_err(|e| format!("Failed to lock executions: {}", e))?;
        executions.insert(
            execution_id.clone(),
            (cancel_token.clone(), progress_state.clone()),
        );
    }

    // Execute and clean up on exit
    let result = execute_udf_inner(
        &execution_id,
        request,
        state,
        cancel_token,
        progress_state,
    );

    // Unregister execution when done
    {
        if let Ok(mut executions) = active_executions.executions.write() {
            executions.remove(&execution_id);
        }
    }

    result
}

/// Inner execution logic (separated for cleanup handling)
fn execute_udf_inner(
    execution_id: &str,
    request: ExecuteUdfRequest,
    state: State<'_, Mutex<ComputeState>>,
    _cancel_token: Arc<CancellationToken>,
    _progress_state: Arc<ProgressState>,
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
    let loader = DataForgeCurveLoader::new(db, blobs_dir.clone());

    // TODO: Pass cancel_token and progress_state to engine.execute
    // when we add async execution support

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

        let mnemonic = output.curve_data.mnemonic.clone();
        let warnings = output.warnings.clone();

        // Optionally save the result back to DataForge
        let (saved, output_curve_id) = if request.save_result {
            // Note: For saving, we would need a read-write connection
            // For now, we log that saving was requested but not performed
            // because the DB is opened read-only
            info!("💾 Save requested for output curve: {}", mnemonic);

            // In a full implementation, we would:
            // 1. Open a read-write connection to the DB
            // 2. Use OutputWriter to write the blob and register the curve
            // For MVP, we indicate save was requested
            (false, None)
        } else {
            (false, None)
        };

        Ok(ExecuteUdfResult {
            success: true,
            execution_id: execution_id.to_string(),
            output_mnemonic: Some(mnemonic),
            output_curve_id,
            output_data: Some(output_data),
            warnings,
            error: None,
            saved,
        })
    } else {
        Ok(ExecuteUdfResult {
            success: false,
            execution_id: execution_id.to_string(),
            output_mnemonic: None,
            output_curve_id: None,
            output_data: None,
            warnings: Vec::new(),
            error: result.record.error_message,
            saved: false,
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

// ==== Provenance Commands ====

/// Response type for curve provenance query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveProvenanceResponse {
    pub id: String,
    pub udf_id: String,
    pub udf_version: String,
    pub inputs: Vec<InputReferenceResponse>,
    pub parameters: serde_json::Value,
    pub output_curve_id: Option<String>,
    pub output_parquet_hash: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub compute_app_version: String,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputReferenceResponse {
    pub curve_id: String,
    pub mnemonic: String,
    pub parquet_hash: String,
    pub version: i64,
}

/// Get provenance information for a derived curve
#[tauri::command]
pub fn get_curve_provenance(
    curve_id: String,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<Option<CurveProvenanceResponse>, String> {
    let state = state.lock().expect("Failed to lock state");
    let db = state.db.as_ref().ok_or("Not connected to DataForge")?;

    // First check if this curve is derived
    let (is_derived, source_execution_id): (bool, Option<String>) = db
        .query_row(
            "SELECT COALESCE(is_derived, 0), source_execution_id FROM curves WHERE id = ?1",
            [&curve_id],
            |row| {
                let is_derived: i64 = row.get(0)?;
                Ok((is_derived != 0, row.get(1)?))
            },
        )
        .map_err(|e| format!("Curve not found: {}", e))?;

    if !is_derived {
        return Ok(None);
    }

    let execution_id = source_execution_id
        .ok_or("Derived curve has no execution ID")?;

    // Query the execution record
    let record: CurveProvenanceResponse = db
        .query_row(
            r#"
            SELECT id, udf_id, udf_version, inputs, parameters,
                   output_curve_id, output_parquet_hash,
                   started_at, completed_at, compute_app_version,
                   status, error_message
            FROM execution_records
            WHERE id = ?1
            "#,
            [&execution_id],
            |row| {
                let inputs_json: String = row.get(3)?;
                let params_json: String = row.get(4)?;

                Ok(CurveProvenanceResponse {
                    id: row.get(0)?,
                    udf_id: row.get(1)?,
                    udf_version: row.get(2)?,
                    inputs: serde_json::from_str(&inputs_json).unwrap_or_default(),
                    parameters: serde_json::from_str(&params_json).unwrap_or(serde_json::json!({})),
                    output_curve_id: row.get(5)?,
                    output_parquet_hash: row.get(6)?,
                    started_at: row.get(7)?,
                    completed_at: row.get(8)?,
                    compute_app_version: row.get(9)?,
                    status: row.get(10)?,
                    error_message: row.get(11)?,
                })
            },
        )
        .map_err(|e| format!("Execution record not found: {}", e))?;

    Ok(Some(record))
}

// ==== Async Execution Commands ====

/// Response for execution progress query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub execution_id: String,
    pub progress: u8,
    pub message: Option<String>,
    pub is_cancelled: bool,
}

/// Get the progress of an active execution
#[tauri::command]
pub fn get_execution_progress(
    execution_id: String,
    active_executions: State<'_, ActiveExecutions>,
) -> Result<Option<ExecutionProgress>, String> {
    let executions = active_executions
        .executions
        .read()
        .map_err(|e| format!("Failed to lock executions: {}", e))?;

    if let Some((cancel_token, progress_state)) = executions.get(&execution_id) {
        Ok(Some(ExecutionProgress {
            execution_id,
            progress: progress_state.get_progress(),
            message: progress_state.get_message(),
            is_cancelled: cancel_token.is_cancelled(),
        }))
    } else {
        Ok(None)
    }
}

/// Cancel an active execution
#[tauri::command]
pub fn cancel_execution(
    execution_id: String,
    active_executions: State<'_, ActiveExecutions>,
) -> Result<bool, String> {
    let executions = active_executions
        .executions
        .read()
        .map_err(|e| format!("Failed to lock executions: {}", e))?;

    if let Some((cancel_token, _)) = executions.get(&execution_id) {
        cancel_token.cancel();
        info!("🛑 Cancellation requested for execution: {}", execution_id);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// List all active executions
#[tauri::command]
pub fn list_active_executions(
    active_executions: State<'_, ActiveExecutions>,
) -> Result<Vec<ExecutionProgress>, String> {
    let executions = active_executions
        .executions
        .read()
        .map_err(|e| format!("Failed to lock executions: {}", e))?;

    let result: Vec<ExecutionProgress> = executions
        .iter()
        .map(|(id, (cancel_token, progress_state))| ExecutionProgress {
            execution_id: id.clone(),
            progress: progress_state.get_progress(),
            message: progress_state.get_message(),
            is_cancelled: cancel_token.is_cancelled(),
        })
        .collect();

    Ok(result)
}

// ==== Save Output Curve Command ====

/// Request to save an output curve to DataForge
#[derive(Debug, Clone, Deserialize)]
pub struct SaveOutputCurveRequest {
    pub execution_id: String,
    pub well_id: String,
    pub workspace_id: String,
    pub mnemonic: Option<String>,
    pub output_data: Vec<CurveDataPoint>,
}

/// Response from saving an output curve
#[derive(Debug, Clone, Serialize)]
pub struct SaveOutputCurveResponse {
    pub success: bool,
    pub curve_id: Option<String>,
    pub error: Option<String>,
}

/// Save the output of a UDF execution as a new curve in DataForge
#[tauri::command]
pub fn save_output_curve(
    request: SaveOutputCurveRequest,
    state: State<'_, Mutex<ComputeState>>,
) -> Result<SaveOutputCurveResponse, String> {
    use arrow::array::Float64Array;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::ArrowWriter;
    use parquet::basic::Compression;
    use parquet::file::properties::WriterProperties;
    use sha2::{Digest, Sha256};
    use std::fs;

    let state = state.lock().expect("Failed to lock state");
    let data_dir = state
        .dataforge_data_dir
        .as_ref()
        .ok_or("DataForge data directory not set")?;

    // We need to open a read-write connection for saving
    let db_path = data_dir.join("dataforge.db");
    let db = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database for writing: {}", e))?;

    let mnemonic = request
        .mnemonic
        .unwrap_or_else(|| format!("DERIVED_{}", &request.execution_id[..8]));

    // Generate curve ID
    let curve_id = Uuid::new_v4();

    // Create parquet data
    let depths: Vec<f64> = request.output_data.iter().map(|p| p.depth).collect();
    let values: Vec<Option<f64>> = request.output_data.iter().map(|p| p.value).collect();

    // Build Arrow arrays
    let depth_array = Float64Array::from(depths.clone());
    let value_array = Float64Array::from(values.clone());

    // Schema: DEPTH, {mnemonic}
    let schema = Arc::new(Schema::new(vec![
        Field::new("DEPTH", DataType::Float64, false),
        Field::new(&mnemonic, DataType::Float64, true),
    ]));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(depth_array), Arc::new(value_array)],
    )
    .map_err(|e| format!("Failed to create record batch: {}", e))?;

    // Write parquet to buffer
    let mut buf = Vec::new();
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();
    let mut writer = ArrowWriter::try_new(&mut buf, schema, Some(props))
        .map_err(|e| format!("Failed to create parquet writer: {}", e))?;
    writer
        .write(&batch)
        .map_err(|e| format!("Failed to write parquet: {}", e))?;
    writer
        .close()
        .map_err(|e| format!("Failed to close parquet writer: {}", e))?;

    // Calculate hash
    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let hash = format!("{:x}", hasher.finalize());

    // Store blob
    let blobs_dir = data_dir.join("blobs");
    let blob_path = blobs_dir
        .join(&hash[0..2])
        .join(&hash[2..4])
        .join(format!("{}.parquet", &hash));

    fs::create_dir_all(blob_path.parent().unwrap())
        .map_err(|e| format!("Failed to create blob directory: {}", e))?;
    fs::write(&blob_path, &buf).map_err(|e| format!("Failed to write blob: {}", e))?;

    // Register blob
    db.execute(
        "INSERT OR IGNORE INTO blob_registry (hash, size_bytes) VALUES (?1, ?2)",
        rusqlite::params![hash, buf.len() as i64],
    )
    .map_err(|e| format!("Failed to register blob: {}", e))?;

    // Calculate statistics
    let valid_values: Vec<f64> = values.iter().filter_map(|v| *v).collect();
    let (min_val, max_val, mean_val) = if valid_values.is_empty() {
        (None, None, None)
    } else {
        let min = valid_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = valid_values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let mean = valid_values.iter().sum::<f64>() / valid_values.len() as f64;
        (Some(min), Some(max), Some(mean))
    };

    let null_count = values.iter().filter(|v| v.is_none()).count() as i64;
    let min_depth = depths.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_depth = depths.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Insert curve record
    db.execute(
        r#"
        INSERT INTO curves (
            id, well_id, mnemonic,
            native_top_depth, native_bottom_depth, native_sample_count,
            min_value, max_value, mean_value, null_count,
            native_parquet_hash, quality_flag, is_derived, source_execution_id,
            created_by
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        "#,
        rusqlite::params![
            curve_id.to_string(),
            request.well_id,
            mnemonic,
            min_depth,
            max_depth,
            depths.len() as i64,
            min_val,
            max_val,
            mean_val,
            null_count,
            hash,
            "derived",
            true,
            request.execution_id,
            "DataForge Compute"
        ],
    )
    .map_err(|e| format!("Failed to insert curve: {}", e))?;

    info!(
        "💾 Saved derived curve {} ({}) with {} points",
        curve_id,
        mnemonic,
        depths.len()
    );

    Ok(SaveOutputCurveResponse {
        success: true,
        curve_id: Some(curve_id.to_string()),
        error: None,
    })
}
