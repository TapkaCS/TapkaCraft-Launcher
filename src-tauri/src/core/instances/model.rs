//! The `instance.json` data contract. Field names/casing intentionally
//! mirror the JSON example in the master spec's "INSTANCE METADATA"
//! section, and the frontend's `InstanceMeta` TypeScript type (see
//! `src/types/instance.ts`) mirrors this struct - both sides describe the
//! same document.

// Not constructed by any command yet - `InstanceService` (Phase 2) is the
// real caller. Only this file's tests exercise it so far, which trips
// `dead_code` under a plain `cargo build`. Remove once Phase 2 lands.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMeta {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderConfig,
    pub java: JavaSettings,

    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub playtime_seconds: u64,
    #[serde(default)]
    pub launch_count: u32,
    /// ISO-8601 timestamp, kept as an opaque string at the model boundary -
    /// this crate does not need to reason about time zones, only persist
    /// and echo back what was recorded.
    #[serde(default)]
    pub last_played: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderConfig {
    #[serde(rename = "type")]
    pub kind: LoaderKind,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    Neoforge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaSettings {
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact example from the master spec's "INSTANCE METADATA"
    /// section must deserialize cleanly into this struct.
    #[test]
    fn parses_the_spec_example() {
        let json = r#"{
            "id": "performance-1-21",
            "name": "Performance",
            "minecraftVersion": "1.21.1",
            "loader": {
                "type": "fabric",
                "version": "latest"
            },
            "java": {
                "memoryMinMb": 2048,
                "memoryMaxMb": 8192
            }
        }"#;

        let meta: InstanceMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.id, "performance-1-21");
        assert_eq!(meta.minecraft_version, "1.21.1");
        assert_eq!(meta.loader.kind, LoaderKind::Fabric);
        assert_eq!(meta.loader.version, "latest");
        assert_eq!(meta.java.memory_min_mb, 2048);
        assert_eq!(meta.java.memory_max_mb, 8192);
        // Fields absent from the minimal spec example must fall back to
        // sane defaults instead of failing to parse.
        assert_eq!(meta.playtime_seconds, 0);
        assert_eq!(meta.launch_count, 0);
        assert!(meta.last_played.is_none());
    }

    #[test]
    fn round_trips_through_json() {
        let meta = InstanceMeta {
            id: "vanilla-1-21".into(),
            name: "Vanilla".into(),
            minecraft_version: "1.21.1".into(),
            loader: LoaderConfig {
                kind: LoaderKind::Vanilla,
                version: "1.21.1".into(),
            },
            java: JavaSettings {
                memory_min_mb: 1024,
                memory_max_mb: 4096,
            },
            icon: Some("vanilla.png".into()),
            favorite: true,
            playtime_seconds: 3600,
            launch_count: 4,
            last_played: Some("2026-09-09T12:00:00Z".into()),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let parsed: InstanceMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, parsed);
    }
}
