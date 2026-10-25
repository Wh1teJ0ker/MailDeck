pub mod app_settings;
pub mod commands;
pub mod crypto;
pub mod credentials;
pub mod error;
pub mod mail;
pub mod provider;
pub mod state;
pub mod store;
pub mod sync;
pub mod updater;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_path = dir.join("maildeck.db");
            // 启动即打开一次:确保迁移完成后再对外服务
            let _ = store::Store::open(&db_path)?;
            app.manage(state::AppState { db_path, app_dir: dir });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_account,
            commands::list_accounts,
            commands::list_providers,
            commands::list_mailboxes,
            commands::remove_account,
            commands::set_account_enabled,
            commands::update_account,
            commands::sync_account,
            commands::sync_all,
            commands::list_messages,
            commands::open_message,
            commands::set_seen,
            commands::mark_all_seen,
            commands::unread_counts,
            commands::get_app_settings,
            commands::set_background,
            commands::upload_background,
            commands::set_glass_tone,
            commands::get_gmail_client_id,
            commands::set_gmail_client_id,
            commands::add_account_gmail,
            commands::get_gmail_client_id,
            commands::set_gmail_client_id,
            commands::add_account_gmail,
            commands::read_custom_background,
            commands::cred_file_status,
            commands::unlock_cred_file,
            commands::lock_cred_file,
            commands::set_credential_store,
            commands::app_version,
            commands::check_update,
            commands::install_update,
            commands::relaunch,
        ])
        .run(tauri::generate_context!())
        .expect("MailDeck 启动失败");
}
