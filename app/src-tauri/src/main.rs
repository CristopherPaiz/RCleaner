// Sin consola en release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod space;
mod state;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::system_info,
            commands::drives,
            commands::history,
            commands::list_modes,
            commands::save_mode,
            commands::delete_mode,
            commands::list_modules,
            commands::get_settings,
            commands::save_settings,
            commands::pick_folder,
            commands::scan,
            commands::clean,
            commands::cancel,
            commands::reveal,
            commands::restart_as_admin,
            space::disk_scan,
            space::disk_summary,
            space::disk_view,
            space::disk_children,
            space::disk_crumbs,
            space::disk_largest,
            space::disk_reveal,
            space::disk_delete,
            space::dupes_scan,
            space::dupes_report,
            space::dupes_reveal,
            space::dupes_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar RCleaner");
}
