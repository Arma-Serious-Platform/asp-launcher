mod arma;
mod commands;
mod ftp;
mod settings;

use commands::AppState;
use settings::Settings;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let settings = Settings::load(app.handle());
            app.manage(AppState::new(settings));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::detect_arma_path,
            commands::validate_arma_path,
            commands::launch_game,
            commands::ftp_test_connection,
            commands::ftp_list_mods,
            commands::sync_status,
            commands::start_sync,
            commands::cancel_sync,
            commands::fetch_weekends,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
