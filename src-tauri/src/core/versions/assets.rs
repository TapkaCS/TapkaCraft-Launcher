//! The asset index: the list of every loose asset (sounds, textures,
//! language files...) a version needs, each addressed by its SHA1 hash
//! rather than a path. Mojang's CDN and our local `assets/objects` cache
//! both lay these out the same way: `objects/<first 2 hex chars>/<hash>`.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::VersionError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

impl AssetObject {
    /// Path relative to the shared assets dir, e.g. `objects/3f/3f2504e...`.
    pub fn relative_path(&self) -> String {
        format!("objects/{}/{}", &self.hash[0..2], self.hash)
    }

    pub fn download_url(&self) -> String {
        format!(
            "https://resources.download.minecraft.net/{}/{}",
            &self.hash[0..2],
            self.hash
        )
    }
}

const ASSET_INDEX_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn fetch_asset_index(
    client: &reqwest::Client,
    url: &str,
) -> Result<AssetIndex, VersionError> {
    let response = client
        .get(url)
        .timeout(ASSET_INDEX_TIMEOUT)
        .send()
        .await
        .map_err(|err| VersionError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(VersionError::Network(format!(
            "asset index request failed with HTTP {}",
            response.status()
        )));
    }

    response
        .json::<AssetIndex>()
        .await
        .map_err(|err| VersionError::Parse(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_uses_the_first_two_hex_chars_as_a_bucket() {
        let obj = AssetObject {
            hash: "3f2504e04f8964a4d5aa9d7d5a5b6c8ef4d1a1a1".to_string(),
            size: 100,
        };
        assert_eq!(
            obj.relative_path(),
            "objects/3f/3f2504e04f8964a4d5aa9d7d5a5b6c8ef4d1a1a1"
        );
        assert_eq!(
            obj.download_url(),
            "https://resources.download.minecraft.net/3f/3f2504e04f8964a4d5aa9d7d5a5b6c8ef4d1a1a1"
        );
    }

    #[test]
    fn parses_a_real_shaped_asset_index() {
        let json = r#"{
            "objects": {
                "icons/icon_16x16.png": { "hash": "bdf48ef6b5d0d23bbb02e17d04865216179f510a", "size": 3665 },
                "minecraft/sounds/random/pop.ogg": { "hash": "6ffe6d189bdf5a1c8b4e5b5a1a1a1a1a1a1a1a1a", "size": 6155 }
            }
        }"#;
        let index: AssetIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.objects.len(), 2);
        assert_eq!(index.objects["icons/icon_16x16.png"].size, 3665);
    }
}
