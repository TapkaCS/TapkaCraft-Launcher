//! `InstanceService` (Phase 2).
//!
//! Owns instance lifecycle - create/list/update/delete, each instance
//! isolated under its own directory (config, mods, saves, resourcepacks,
//! shaderpacks, screenshots, `options.txt`) while assets/libraries/runtimes/
//! versions stay shared - and reading/writing each instance's
//! `instance.json`. Not implemented yet: only the metadata contract
//! (`model`) exists so far, matching the schema the master spec defines
//! under "INSTANCE METADATA". The frontend's Phase 1 profile list uses this
//! same shape for its mock data so switching it over to real instances
//! later is a data-source change, not a UI rewrite.

pub mod model;
