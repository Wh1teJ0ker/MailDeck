pub mod commands;
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
            let store = store::Store::open(&dir.join("maildeck.db"))?;
            app.manage(state::AppState {
                db: std::sync::Arc::new(tokio::sync::Mutex::new(store)),
            });
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
            commands::app_version,
            commands::check_update,
            commands::install_update,
            commands::relaunch,
        ])
        .run(tauri::generate_context!())
        .expect("MailDeck 启动失败");
}
