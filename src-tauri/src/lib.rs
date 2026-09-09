mod commands;
mod core;
#[cfg(test)]
mod test_support;

use core::paths::AppPaths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let args: Vec<String> = std::env::args().collect();
    let app_paths = AppPaths::resolve(&exe_dir, &args);
    // Explicit User-Agent: reqwest sends none by default, and Mojang's
    // account/identity endpoints (api.minecraftservices.com in particular)
    // sit behind bot-protection that has been observed to reject requests
    // with no identifiable client as 403 Forbidden, even when the request
    // itself is otherwise well-formed.
    let http_client = reqwest::Client::builder()
        .user_agent(concat!("TapkaCraft-Launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("building the shared HTTP client with just a User-Agent set should never fail");

    tauri::Builder::default()
        .manage(app_paths)
        .manage(http_client)
        .manage(commands::accounts::ActiveSession::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_launcher_version,
            commands::instances::list_instances,
            commands::instances::create_instance,
            commands::instances::update_instance,
            commands::instances::delete_instance,
            commands::instances::open_instance_folder,
            commands::versions::get_version_manifest,
            commands::versions::install_instance,
            commands::java::detect_java_runtimes,
            commands::system::get_memory_suggestion,
            commands::system::get_gpu_info,
            commands::accounts::begin_sign_in,
            commands::accounts::try_restore_session,
            commands::accounts::list_accounts,
            commands::accounts::active_account,
            commands::accounts::switch_account,
            commands::accounts::sign_out,
            commands::accounts::remove_account,
            commands::launch::launch_instance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TapkaCraft Launcher");
}
