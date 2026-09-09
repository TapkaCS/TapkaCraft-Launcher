mod commands;
mod core;

use core::paths::AppPaths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let args: Vec<String> = std::env::args().collect();
    let app_paths = AppPaths::resolve(&exe_dir, &args);

    tauri::Builder::default()
        .manage(app_paths)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_launcher_version,
            commands::instances::list_instances,
            commands::instances::create_instance,
            commands::instances::update_instance,
            commands::instances::delete_instance,
            commands::instances::open_instance_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TapkaCraft Launcher");
}
