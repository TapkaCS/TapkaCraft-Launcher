//! `DownloadManager` (Phase 3).
//!
//! The single place network fetches for game files, libraries, assets,
//! runtimes and content (mods/modpacks/shaders) go through - the UI layer
//! never issues fetches directly. Provides a concurrency-limited queue with
//! per-file and aggregate progress, retries with timeout, cancellation,
//! SHA1/SHA256 hash verification against Mojang/Modrinth-supplied hashes,
//! and atomic writes (temp file + rename) so a crash or cancel can never
//! leave a half-written file mistaken for a real one.
