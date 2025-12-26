mod commands;
mod compute;

use commands::{ActiveExecutions, ComputeState};
use log::info;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .manage(Mutex::new(ComputeState::default()))
        .manage(ActiveExecutions::default())
        .invoke_handler(tauri::generate_handler![
            // DataForge data access (read-only)
            commands::get_dataforge_status,
            commands::list_workspaces,
            commands::list_wells,
            commands::list_curves,
            commands::list_all_curves_for_workspace,
            commands::get_curve_data,
            // Legacy computations (to be deprecated)
            commands::compute_moving_average,
            // UDF system
            commands::list_providers,
            commands::list_udfs,
            commands::get_udf_parameters,
            commands::execute_udf,
            commands::validate_udf_parameters,
            // Save output
            commands::save_output_curve,
            // Provenance
            commands::get_curve_provenance,
            // Progress and cancellation
            commands::get_execution_progress,
            commands::cancel_execution,
            commands::list_active_executions,
        ])
        .setup(|app| {
            info!("🚀 Initializing DataForge Compute");

            // Get the main window (it's hidden initially)
            let window = app.get_webview_window("main").unwrap();

            // Initialize state - locate DataForge's data directory
            let state = app.state::<Mutex<ComputeState>>();
            let init_result = {
                let mut state = state.lock().expect("Failed to lock compute state");
                state.initialize()
            };

            match init_result {
                Ok(_) => {
                    info!("✅ Compute state initialized successfully");
                }
                Err(e) => {
                    log::error!("❌ Failed to initialize compute state: {}", e);
                }
            }

            // Open devtools in debug mode
            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            // Show window after initialization
            info!("👁️ Showing main window");
            window.show().expect("Failed to show main window");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
