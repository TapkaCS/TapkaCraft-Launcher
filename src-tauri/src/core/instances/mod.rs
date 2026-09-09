//! `InstanceService`: instance lifecycle (create/list/update/delete), each
//! instance isolated under its own directory (`instance.json`, `mods/`,
//! `saves/`, `resourcepacks/`, `shaderpacks/`, `screenshots/` - `config/`
//! and `options.txt` are written by the game itself once launching exists,
//! not pre-created here) while assets/libraries/runtimes/versions stay
//! shared under `AppPaths` - matching the "INSTANCE SYSTEM" section of the
//! project plan. `crate::commands::instances` is the thin Tauri IPC layer
//! on top of this; the frontend never touches the filesystem directly.

pub mod model;
pub mod service;
