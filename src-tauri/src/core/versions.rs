//! `VersionService` (Phase 3, extended through Phase 5/8/11).
//!
//! Fetches and caches the Mojang version manifest, resolves a single
//! version's metadata JSON (including legacy versions that `inheritsFrom`
//! an older JSON), and evaluates the OS/arch/feature rule blocks Mojang
//! attaches to libraries and arguments - launch commands are not the same
//! shape across all Minecraft versions and this module is what accounts
//! for that. Also home to the pluggable loader installers behind one
//! `LoaderInstaller` trait: `VanillaLoader`, `FabricLoader`, `QuiltLoader`,
//! `ForgeLoader`, `NeoForgeLoader`.
