//! Launcher core: platform- and UI-independent services that make up
//! TapkaCraft Launcher's actual functionality (see `ARCHITECTURE` in the
//! project plan). The frontend never contains launch logic, network calls
//! to Mojang/Microsoft/Modrinth, or filesystem writes directly - it only
//! reaches these services through `crate::commands`.
//!
//! `paths` and `instances` are real, working, tested services wired up to
//! Tauri commands as of Phase 2. Every other submodule currently holds
//! nothing but a doc comment describing what will live there, so later
//! phases have a stable, pre-agreed home to land in instead of reshuffling
//! the tree - none of them are referenced by any command yet.

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
