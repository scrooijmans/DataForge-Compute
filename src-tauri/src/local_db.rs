//! Local database module for DataForge Compute.
//!
//! This module provides a separate local SQLite database for storing user preferences
//! and state that is specific to DataForge Compute, such as workspace chart layouts.
//!
//! This is separate from the shared DataForge database (which is read-only from Compute's
//! perspective) and is used for:
//! - Chart layout persistence per workspace
//! - User preferences
//! - Local execution history
//!
//! The database is stored at: `<app_data_dir>/compute_local.db`

use chrono::Utc;
use log::info;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

/// Local database state manager
pub struct LocalDatabase {
    /// SQLite connection for local storage
    pub conn: Connection,
    /// Path to the database file
    pub db_path: PathBuf,
}

/// Chart layout stored in the local database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartLayout {
    pub id: String,
    pub workspace_id: String,
    pub layout_json: String,
    pub version: i32,
    pub sync_version: i32,
    pub sync_status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl LocalDatabase {
    /// Create a new local database instance
    pub fn new(app_data_dir: &PathBuf) -> anyhow::Result<Self> {
        let db_path = app_data_dir.join("compute_local.db");

        info!("📂 Opening local database at: {:?}", db_path);

        // Create the connection (will create file if it doesn't exist)
        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        let mut db = Self { conn, db_path };

        // Run migrations
        db.migrate()?;

        Ok(db)
    }

    /// Run database migrations
    fn migrate(&mut self) -> anyhow::Result<()> {
        info!("📦 Running local database migrations...");

        // Create migrations tracking table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            )",
            [],
        )?;

        // Get current schema version
        let current_version: i32 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        info!("📊 Current schema version: {}", current_version);

        // Migration 1: Create chart_layouts table
        if current_version < 1 {
            info!("🔄 Applying migration 1: Create chart_layouts table");
            self.conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS chart_layouts (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    layout_json TEXT NOT NULL,
                    version INTEGER NOT NULL DEFAULT 1,
                    sync_version INTEGER NOT NULL DEFAULT 0,
                    sync_status TEXT NOT NULL DEFAULT 'pending',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    UNIQUE(workspace_id)
                );

                CREATE INDEX IF NOT EXISTS idx_chart_layouts_workspace
                ON chart_layouts(workspace_id);

                CREATE INDEX IF NOT EXISTS idx_chart_layouts_sync_status
                ON chart_layouts(sync_status);

                INSERT INTO schema_migrations (version, applied_at) VALUES (1, datetime('now'));
                "#,
            )?;
            info!("✅ Migration 1 applied successfully");
        }

        info!("✅ All migrations applied");
        Ok(())
    }

    /// Save or update a workspace layout
    pub fn save_workspace_layout(
        &self,
        workspace_id: &str,
        layout_json: &str,
    ) -> anyhow::Result<ChartLayout> {
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();

        // Upsert: insert or update on conflict
        self.conn.execute(
            r#"
            INSERT INTO chart_layouts (id, workspace_id, layout_json, version, sync_version, sync_status, created_at, updated_at)
            VALUES (?1, ?2, ?3, 1, 0, 'pending', ?4, ?4)
            ON CONFLICT(workspace_id) DO UPDATE SET
                layout_json = excluded.layout_json,
                updated_at = excluded.updated_at,
                sync_status = 'pending'
            "#,
            params![id, workspace_id, layout_json, now],
        )?;

        // Return the saved/updated layout
        self.get_workspace_layout(workspace_id)?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve saved layout"))
    }

    /// Get a workspace layout by workspace ID
    pub fn get_workspace_layout(&self, workspace_id: &str) -> anyhow::Result<Option<ChartLayout>> {
        let layout = self.conn.query_row(
            r#"
            SELECT id, workspace_id, layout_json, version, sync_version, sync_status, created_at, updated_at
            FROM chart_layouts
            WHERE workspace_id = ?1
            "#,
            params![workspace_id],
            |row| {
                Ok(ChartLayout {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    layout_json: row.get(2)?,
                    version: row.get(3)?,
                    sync_version: row.get(4)?,
                    sync_status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        ).optional()?;

        Ok(layout)
    }

    /// Delete a workspace layout
    pub fn delete_workspace_layout(&self, workspace_id: &str) -> anyhow::Result<bool> {
        let rows_affected = self.conn.execute(
            "DELETE FROM chart_layouts WHERE workspace_id = ?1",
            params![workspace_id],
        )?;

        Ok(rows_affected > 0)
    }

    /// List all workspace layouts (for debugging/sync)
    #[allow(dead_code)]
    pub fn list_workspace_layouts(&self) -> anyhow::Result<Vec<ChartLayout>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, workspace_id, layout_json, version, sync_version, sync_status, created_at, updated_at
            FROM chart_layouts
            ORDER BY updated_at DESC
            "#,
        )?;

        let layouts = stmt.query_map([], |row| {
            Ok(ChartLayout {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                layout_json: row.get(2)?,
                version: row.get(3)?,
                sync_version: row.get(4)?,
                sync_status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        layouts.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

/// Thread-safe wrapper for LocalDatabase
pub struct LocalDbState {
    pub db: Mutex<LocalDatabase>,
}

impl LocalDbState {
    pub fn new(app_data_dir: &PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            db: Mutex::new(LocalDatabase::new(app_data_dir)?),
        })
    }
}
