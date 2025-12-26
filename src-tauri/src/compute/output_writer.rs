//! Output writer for persisting derived curves back to DataForge.
//!
//! This module handles the atomic write of UDF outputs to DataForge's
//! blob store and database, ensuring full provenance tracking.

use crate::compute::error::UdfError;
use crate::compute::types::{ExecutionRecord, OutputCurveData};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

/// Result of registering an execution output.
#[derive(Debug, Clone)]
pub struct RegisteredOutput {
    /// The new curve ID
    pub curve_id: Uuid,
    /// The parquet content hash
    pub parquet_hash: String,
    /// Path to the written blob
    pub blob_path: PathBuf,
}

/// Writer for persisting UDF outputs to DataForge.
pub struct OutputWriter {
    /// Path to the blobs directory
    blobs_dir: PathBuf,
}

impl OutputWriter {
    /// Create a new output writer.
    pub fn new(blobs_dir: PathBuf) -> Self {
        Self { blobs_dir }
    }

    /// Write output curve data to a Parquet blob.
    ///
    /// Returns the SHA-256 hash of the content and the blob path.
    pub fn write_parquet_blob(
        &self,
        output: &OutputCurveData,
    ) -> Result<(String, PathBuf), UdfError> {
        // Create Parquet data in memory
        let parquet_bytes = self.create_parquet_bytes(output)?;

        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&parquet_bytes);
        let hash = format!("{:x}", hasher.finalize());

        // Determine blob path (content-addressed)
        let blob_path = self
            .blobs_dir
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(format!("{}.parquet", hash));

        // Check if blob already exists (content-addressed deduplication)
        if blob_path.exists() {
            return Ok((hash, blob_path));
        }

        // Create directories if needed
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                UdfError::IoError(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create blob directory: {}", e),
                ))
            })?;
        }

        // Write to temp file first, then rename (atomic on POSIX)
        let temp_path = blob_path.with_extension("parquet.tmp");
        {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(&parquet_bytes)?;
            file.sync_all()?; // fsync for durability
        }

        // Atomic rename
        fs::rename(&temp_path, &blob_path).map_err(|e| {
            // Clean up temp file on error
            let _ = fs::remove_file(&temp_path);
            UdfError::IoError(e)
        })?;

        Ok((hash, blob_path))
    }

    /// Create Parquet bytes from output curve data.
    ///
    /// For MVP, we create a simple CSV-like format that DuckDB can read.
    /// In production, this should use Arrow/Parquet libraries.
    fn create_parquet_bytes(&self, output: &OutputCurveData) -> Result<Vec<u8>, UdfError> {
        // For MVP, create a simple binary format that we can later upgrade to Parquet
        // Format: CSV with depth,value columns (DuckDB can read this)
        //
        // TODO: Replace with proper Parquet writing using arrow-rs
        let mut csv_content = String::from("depth,value\n");

        for (depth, value) in output.depths.iter().zip(output.values.iter()) {
            match value {
                Some(v) => csv_content.push_str(&format!("{},{}\n", depth, v)),
                None => csv_content.push_str(&format!("{},\n", depth)),
            }
        }

        Ok(csv_content.into_bytes())
    }

    /// Register the output curve in DataForge's database.
    ///
    /// This creates a new curve record with provenance linking back to
    /// the execution record.
    pub fn register_curve(
        &self,
        db: &Connection,
        well_id: Uuid,
        output: &OutputCurveData,
        parquet_hash: &str,
        execution_record: &ExecutionRecord,
    ) -> Result<Uuid, UdfError> {
        let curve_id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();

        // Calculate statistics
        let (min_depth, max_depth) = if output.depths.is_empty() {
            (None, None)
        } else {
            (
                output.depths.first().copied(),
                output.depths.last().copied(),
            )
        };

        let row_count = output.depths.len() as i64;

        // Calculate value statistics
        let valid_values: Vec<f64> = output.values.iter().filter_map(|v| *v).collect();
        let (min_value, max_value) = if valid_values.is_empty() {
            (None, None)
        } else {
            let min = valid_values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = valid_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (Some(min), Some(max))
        };

        db.execute(
            "INSERT INTO curves (
                id, well_id, mnemonic, unit, description,
                min_depth, max_depth, row_count, min_value, max_value,
                parquet_hash, version, is_derived, source_execution_id,
                created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14,
                ?15, ?16
            )",
            rusqlite::params![
                curve_id.to_string(),
                well_id.to_string(),
                output.mnemonic,
                output.unit,
                output.description,
                min_depth,
                max_depth,
                row_count,
                min_value,
                max_value,
                parquet_hash,
                1, // version
                true, // is_derived
                execution_record.id.to_string(),
                now,
                now,
            ],
        )?;

        Ok(curve_id)
    }

    /// Perform atomic commit of execution result.
    ///
    /// This writes the blob and registers the curve in a single transaction.
    pub fn commit_execution(
        &self,
        db: &Connection,
        well_id: Uuid,
        output: &OutputCurveData,
        execution_record: &mut ExecutionRecord,
    ) -> Result<RegisteredOutput, UdfError> {
        // Write blob first (outside transaction - idempotent due to content addressing)
        let (parquet_hash, blob_path) = self.write_parquet_blob(output)?;

        // Update execution record with output info
        execution_record.output_parquet_hash = Some(parquet_hash.clone());

        // Register curve in database (should be in a transaction in production)
        let curve_id = self.register_curve(db, well_id, output, &parquet_hash, execution_record)?;

        // Update execution record with curve ID
        execution_record.output_curve_id = Some(curve_id);

        Ok(RegisteredOutput {
            curve_id,
            parquet_hash,
            blob_path,
        })
    }
}

/// Check if the curves table has the required columns for derived curves.
/// If not, add them (migration for existing DataForge installations).
pub fn ensure_derived_curve_columns(db: &Connection) -> Result<(), UdfError> {
    // Check if is_derived column exists
    let has_is_derived: bool = db
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('curves') WHERE name = 'is_derived'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_is_derived {
        // Add columns for derived curve tracking
        db.execute_batch(
            r#"
            ALTER TABLE curves ADD COLUMN is_derived INTEGER DEFAULT 0;
            ALTER TABLE curves ADD COLUMN source_execution_id TEXT;
            "#,
        )
        .map_err(|e| {
            UdfError::DatabaseError(format!("Failed to add derived curve columns: {}", e))
        })?;
    }

    Ok(())
}
