//! Tauri IPC wrapper around `core::launch`: the command that finally turns
//! a profile + a signed-in account into a running Minecraft process.
//! Verifies the install (reusing the same `install://progress` events the
//! standalone Install button emits), picks a compatible Java runtime, and
//! only then hands off to `LaunchEngine`.

use std::fmt;

use tauri::{AppHandle, Emitter, State};

use super::accounts::ActiveSession;
use crate::core::accounts::token_store::KeyringTokenStore;
use crate::core::accounts::{oauth, service as account_service, AccountError};
use crate::core::downloads::{DownloadEvent, ProgressSink};
use crate::core::instances::model::LoaderKind;
use crate::core::instances::service::{self as instance_service, InstanceError};
use crate::core::java::manifest::RUNTIME_MANIFEST_URL;
use crate::core::java::{detect, install as java_install, JavaInstallError};
use crate::core::launch::{self, LaunchEventSink, LaunchParams};
use crate::core::paths::AppPaths;
use crate::core::versions::install::{ensure_installed, InstallPaths};
use crate::core::versions::manifest::{self, MANIFEST_URL};
use crate::core::versions::VersionError;

const INSTALL_PROGRESS_EVENT: &str = "install://progress";
const LAUNCH_EVENT: &str = "launch://event";

#[derive(Debug)]
pub enum LaunchCommandError {
    Instance(InstanceError),
    Version(VersionError),
    Account(AccountError),
    Launch(launch::LaunchError),
    JavaInstall(JavaInstallError),
    NotSignedIn,
    UnsupportedLoader { loader: &'static str },
}

impl fmt::Display for LaunchCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instance(err) => write!(f, "{err}"),
            Self::Version(err) => write!(f, "{err}"),
            Self::Account(err) => write!(f, "{err}"),
            Self::Launch(err) => write!(f, "{err}"),
            Self::JavaInstall(err) => write!(f, "{err}"),
            Self::NotSignedIn => write!(f, "Sign in with a Microsoft account first."),
            Self::UnsupportedLoader { loader } => write!(
                f,
                "Launching {loader} instances isn't supported yet - only Vanilla can be launched so far."
            ),
        }
    }
}

impl std::error::Error for LaunchCommandError {}

impl From<InstanceError> for LaunchCommandError {
    fn from(err: InstanceError) -> Self {
        Self::Instance(err)
    }
}

impl From<VersionError> for LaunchCommandError {
    fn from(err: VersionError) -> Self {
        Self::Version(err)
    }
}

impl From<AccountError> for LaunchCommandError {
    fn from(err: AccountError) -> Self {
        Self::Account(err)
    }
}

impl From<launch::LaunchError> for LaunchCommandError {
    fn from(err: launch::LaunchError) -> Self {
        Self::Launch(err)
    }
}

impl From<JavaInstallError> for LaunchCommandError {
    fn from(err: JavaInstallError) -> Self {
        Self::JavaInstall(err)
    }
}

impl serde::Serialize for LaunchCommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn loader_label(kind: LoaderKind) -> &'static str {
    match kind {
        LoaderKind::Vanilla => "Vanilla",
        LoaderKind::Fabric => "Fabric",
        LoaderKind::Quilt => "Quilt",
        LoaderKind::Forge => "Forge",
        LoaderKind::Neoforge => "NeoForge",
    }
}

/// Ensures a usable Minecraft session: the cached one from a recent
/// sign-in/launch if present, otherwise a silent refresh under whichever
/// account is active. Never opens a browser - if neither works, the
/// answer is "sign in first", not a background prompt the user didn't ask
/// for.
async fn resolve_session(
    app_paths: &AppPaths,
    client: &reqwest::Client,
    active_session: &ActiveSession,
) -> Result<crate::core::launch::AuthSession, LaunchCommandError> {
    if let Some(session) = active_session.0.lock().await.clone() {
        return Ok(session);
    }

    let accounts_path = account_service::accounts_file_path(&app_paths.settings_dir());
    let active_uuid = account_service::active_account_uuid(&accounts_path)
        .ok_or(LaunchCommandError::NotSignedIn)?;

    let token_store = KeyringTokenStore;
    let refreshed = account_service::silent_refresh(
        &accounts_path,
        &token_store,
        client,
        oauth::MICROSOFT_CLIENT_ID,
        &active_uuid,
    )
    .await
    .map_err(|_| LaunchCommandError::NotSignedIn)?;

    *active_session.0.lock().await = Some(refreshed.clone());
    Ok(refreshed)
}

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    active_session: State<'_, ActiveSession>,
    id: String,
    concurrency: usize,
) -> Result<i32, LaunchCommandError> {
    let instance = instance_service::get(&app_paths.instances_dir(), &id)?;
    if instance.loader.kind != LoaderKind::Vanilla {
        return Err(LaunchCommandError::UnsupportedLoader {
            loader: loader_label(instance.loader.kind),
        });
    }

    let client = http_client.inner().clone();
    let session = resolve_session(&app_paths, &client, &active_session).await?;

    let manifest = manifest::fetch_or_cached(&client, MANIFEST_URL, &app_paths.cache_dir()).await?;
    let install_paths = InstallPaths {
        versions_dir: app_paths.versions_dir(),
        libraries_dir: app_paths.libraries_dir(),
        assets_dir: app_paths.assets_dir(),
    };

    let (install_tx, mut install_rx) = tokio::sync::mpsc::unbounded_channel::<DownloadEvent>();
    let install_forward_app = app.clone();
    let install_forwarder = tokio::spawn(async move {
        while let Some(event) = install_rx.recv().await {
            let _ = install_forward_app.emit(INSTALL_PROGRESS_EVENT, event);
        }
    });
    let installed = ensure_installed(
        &client,
        &install_paths,
        &manifest,
        &instance.minecraft_version,
        concurrency,
        ProgressSink::new(install_tx),
    )
    .await;
    let _ = install_forwarder.await;
    let installed = installed?;

    // Versions old enough to predate the `javaVersion` field (pre-1.17)
    // don't name a component - "jre-legacy" (Java 8) is what Mojang's own
    // launcher falls back to for exactly those, so this does too.
    let (required_major, component) = installed
        .version_info
        .java_version
        .as_ref()
        .map(|req| (req.major_version, req.component.clone()))
        .unwrap_or((8, "jre-legacy".to_string()));

    let runtimes = detect::detect_all(&app_paths.runtimes_dir());
    let runtime = match detect::best_match(&runtimes, required_major) {
        Some(runtime) => runtime.clone(),
        None => {
            // No compatible Java on this machine - fetch and install the
            // matching runtime from Mojang ourselves, streaming progress
            // over the same event stream the version install just used.
            let (java_tx, mut java_rx) = tokio::sync::mpsc::unbounded_channel::<DownloadEvent>();
            let java_forward_app = app.clone();
            let java_forwarder = tokio::spawn(async move {
                while let Some(event) = java_rx.recv().await {
                    let _ = java_forward_app.emit(INSTALL_PROGRESS_EVENT, event);
                }
            });
            let installed_runtime = java_install::install_runtime(
                &client,
                RUNTIME_MANIFEST_URL,
                &app_paths.runtimes_dir(),
                &component,
                required_major,
                concurrency,
                ProgressSink::new(java_tx),
            )
            .await;
            let _ = java_forwarder.await;
            installed_runtime?
        }
    };

    let instance_dir = instance_service::resolve_instance_dir(&app_paths.instances_dir(), &id)?;

    let (launch_tx, mut launch_rx) = tokio::sync::mpsc::unbounded_channel();
    let launch_forward_app = app.clone();
    let launch_forwarder = tokio::spawn(async move {
        while let Some(event) = launch_rx.recv().await {
            let _ = launch_forward_app.emit(LAUNCH_EVENT, event);
        }
    });

    let params = LaunchParams {
        instance_dir: &instance_dir,
        java_settings: &instance.java,
        installed: &installed,
        libraries_dir: &install_paths.libraries_dir,
        assets_dir: &install_paths.assets_dir,
        runtime: &runtime,
        auth: &session,
        launcher_name: "TapkaCraft Launcher",
        launcher_version: env!("CARGO_PKG_VERSION"),
    };

    let started_at = std::time::Instant::now();
    let result = launch::launch(params, LaunchEventSink::new(launch_tx)).await;
    let _ = launch_forwarder.await;

    let playtime_seconds = started_at.elapsed().as_secs();
    let _ = instance_service::record_launch(
        &app_paths.instances_dir(),
        &id,
        playtime_seconds,
        account_service::now_iso8601(),
    );

    Ok(result?)
}
