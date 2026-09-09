//! Evaluates the OS/feature `rules` blocks Mojang attaches to libraries and
//! launch arguments. The algorithm (walk every rule in order, the last one
//! whose conditions match wins, default to "disallowed" when rules exist
//! but none match, "allowed" when the list is empty) is Mojang's own -
//! every third-party launcher implements the same thing, since it is the
//! only way to reproduce the official launcher's behavior.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<RuleOs>,
    #[serde(default)]
    pub features: Option<RuleFeatures>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleOs {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    /// A regex Mojang matches against the OS version string (used for
    /// Windows-10-specific JVM args). Rarely present; not evaluated - we
    /// don't need OS-version-specific tweaks to run correctly, only to
    /// look identical to the official launcher's cosmetics.
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleFeatures {
    #[serde(default)]
    pub is_demo_user: Option<bool>,
    #[serde(default)]
    pub has_custom_resolution: Option<bool>,
    #[serde(default)]
    pub has_quick_plays_support: Option<bool>,
    #[serde(default)]
    pub is_quick_play_singleplayer: Option<bool>,
    #[serde(default)]
    pub is_quick_play_multiplayer: Option<bool>,
    #[serde(default)]
    pub is_quick_play_realms: Option<bool>,
}

/// The subset of "what platform/session are we" that rules can key off.
/// Everything defaults to "not active" - TapkaCraft doesn't support demo
/// mode or quick play yet, so those features are always considered unset.
#[derive(Debug, Clone)]
pub struct RuleContext {
    pub os_name: String,
    pub os_arch: String,
    pub has_custom_resolution: bool,
}

impl RuleContext {
    /// Builds a context for the machine this code is actually running on.
    /// `has_custom_resolution` is a caller-supplied setting, not something
    /// the OS reports.
    pub fn current(has_custom_resolution: bool) -> Self {
        Self {
            os_name: mojang_os_name().to_string(),
            os_arch: mojang_os_arch().to_string(),
            has_custom_resolution,
        }
    }
}

/// Mojang's rules use "osx", not Rust's `cfg!(target_os)` string "macos".
fn mojang_os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        other => other, // "windows" and "linux" already match
    }
}

/// Mojang mostly cares about "x86" (32-bit) as a special case; every other
/// modern architecture rule uses arch strings close enough to Rust's own
/// (`x86_64`, `arm64`) that a couple of aliases cover the ones actually
/// seen in the wild.
fn mojang_os_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "arm64",
        other => other,
    }
}

impl Rule {
    fn matches(&self, ctx: &RuleContext) -> bool {
        if let Some(os) = &self.os {
            if let Some(name) = &os.name {
                if name != &ctx.os_name {
                    return false;
                }
            }
            if let Some(arch) = &os.arch {
                if arch != &ctx.os_arch {
                    return false;
                }
            }
        }
        if let Some(features) = &self.features {
            if let Some(demo) = features.is_demo_user {
                if demo {
                    return false; // TapkaCraft never runs demo sessions
                }
            }
            if let Some(custom_res) = features.has_custom_resolution {
                if custom_res != ctx.has_custom_resolution {
                    return false;
                }
            }
            if features.has_quick_plays_support == Some(true)
                || features.is_quick_play_singleplayer == Some(true)
                || features.is_quick_play_multiplayer == Some(true)
                || features.is_quick_play_realms == Some(true)
            {
                return false; // not supported yet
            }
        }
        true
    }
}

/// `true` if this set of rules permits the item on the current platform.
/// An empty/absent rule list always permits it - Mojang only attaches
/// `rules` to items that need restricting.
pub fn rules_allow(rules: &[Rule], ctx: &RuleContext) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule.matches(ctx) {
            allowed = rule.action == RuleAction::Allow;
        }
    }
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> RuleContext {
        RuleContext {
            os_name: "windows".into(),
            os_arch: "x86_64".into(),
            has_custom_resolution: false,
        }
    }

    #[test]
    fn empty_rules_always_allow() {
        assert!(rules_allow(&[], &ctx()));
    }

    #[test]
    fn simple_os_allow_matches_current_platform() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: Some(RuleOs {
                name: Some("windows".into()),
                arch: None,
                version: None,
            }),
            features: None,
        }];
        assert!(rules_allow(&rules, &ctx()));
    }

    #[test]
    fn simple_os_allow_rejects_other_platforms() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: Some(RuleOs {
                name: Some("osx".into()),
                arch: None,
                version: None,
            }),
            features: None,
        }];
        assert!(!rules_allow(&rules, &ctx()));
    }

    #[test]
    fn allow_all_then_disallow_specific_os_excludes_that_os() {
        // The real shape of e.g. an LWJGL library that ships for every OS
        // except one.
        let rules = vec![
            Rule {
                action: RuleAction::Allow,
                os: None,
                features: None,
            },
            Rule {
                action: RuleAction::Disallow,
                os: Some(RuleOs {
                    name: Some("osx".into()),
                    arch: None,
                    version: None,
                }),
                features: None,
            },
        ];
        assert!(rules_allow(&rules, &ctx()));

        let mac_ctx = RuleContext {
            os_name: "osx".into(),
            ..ctx()
        };
        assert!(!rules_allow(&rules, &mac_ctx));
    }

    #[test]
    fn demo_user_rule_is_never_satisfied() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: None,
            features: Some(RuleFeatures {
                is_demo_user: Some(true),
                ..Default::default()
            }),
        }];
        assert!(!rules_allow(&rules, &ctx()));
    }

    #[test]
    fn custom_resolution_rule_follows_the_context_flag() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: None,
            features: Some(RuleFeatures {
                has_custom_resolution: Some(true),
                ..Default::default()
            }),
        }];
        assert!(!rules_allow(&rules, &ctx()));

        let with_custom_res = RuleContext {
            has_custom_resolution: true,
            ..ctx()
        };
        assert!(rules_allow(&rules, &with_custom_res));
    }

    #[test]
    fn arch_specific_rule_only_matches_that_arch() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: Some(RuleOs {
                name: Some("windows".into()),
                arch: Some("arm64".into()),
                version: None,
            }),
            features: None,
        }];
        assert!(!rules_allow(&rules, &ctx())); // ctx() is x86_64
    }
}
