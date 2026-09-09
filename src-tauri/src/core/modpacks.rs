//! `ModpackService` (Phase 7, extended in Phase 10).
//!
//! Installs `.mrpack` modpacks (local file import or downloaded from
//! Modrinth) into a fresh, isolated instance: resolves and downloads every
//! declared dependency, installs overrides, and never touches another
//! instance's files. Defines the `ContentProvider` abstraction
//! (`ModrinthProvider`, `CurseForgeProvider`, `LocalProvider`) so later
//! providers plug into the same install pipeline instead of each growing
//! their own.
