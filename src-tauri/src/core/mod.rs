//! Launcher core: platform- and UI-independent services that make up
//! TapkaCraft Launcher's actual functionality (see `ARCHITECTURE` in the
//! project plan). The frontend never contains launch logic, network calls
//! to Mojang/Microsoft/Modrinth, or filesystem writes directly - it only
//! reaches these services through `crate::commands`.
//!
//! Every submodule here is a real, working, tested service wired up to
//! Tauri commands - none of this tree is a placeholder for a later phase
//! anymore.

pub mod paths;

pub mod instances;

pub mod accounts;
pub mod downloads;
pub mod java;
pub mod launch;
pub mod loaders;
pub mod modpacks;
pub mod modrinth;
pub mod system;
pub mod versions;
