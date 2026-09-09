mod commands;
mod core;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::get_launcher_version])
        .run(tauri::generate_context!())
        .expect("error while running TapkaCraft Launcher");
}
