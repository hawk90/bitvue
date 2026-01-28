// Tauri commands
mod commands;
mod services;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Initialize application state
  let app_state = AppState::new();

  tauri::Builder::default()
    .manage(app_state)
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_store::Builder::new().build())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::greet,
      commands::file::open_file,
      commands::file::close_file,
      commands::file::get_stream_info,
      commands::file::get_frames,
      commands::thumbnails::get_thumbnails,
      commands::window::close_window,
      commands::recent_files::get_recent_files,
      commands::recent_files::add_recent_file,
      commands::recent_files::clear_recent_files,
      commands::frame::get_decoded_frame,
      commands::frame::get_decoded_frame_yuv,
      commands::analysis::get_frame_analysis,
      commands::analysis::get_coding_flow_analysis,
      commands::analysis::get_residual_analysis,
      commands::analysis::get_deblocking_analysis,
      commands::frame::get_frame_hex_data,
      commands::syntax::get_frame_syntax,
      commands::compare::create_compare_workspace,
      commands::compare::get_aligned_frame,
      commands::compare::set_sync_mode,
      commands::compare::set_manual_offset,
      commands::compare::reset_offset,
      commands::export::export_frames_csv,
      commands::export::export_frames_json,
      commands::export::export_analysis_report,
      commands::quality::calculate_quality_metrics,
      commands::quality::calculate_bd_rate,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
