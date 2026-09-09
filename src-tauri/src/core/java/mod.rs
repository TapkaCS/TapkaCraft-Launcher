//! `JavaManager`: finds Java runtimes on the machine (`PATH`, `JAVA_HOME`,
//! known install locations, the Windows registry, and TapkaCraft's own
//! managed `runtimes/` dir), verifies each one by actually running
//! `java -version` rather than trusting a folder/registry name, and picks
//! the runtime matching what a given Minecraft version requires. Never
//! touches the user's system Java installation, `JAVA_HOME`, or `PATH` -
//! only reads them.
//!
//! Since this phase, it can also install a missing runtime itself: Mojang
//! distributes prebuilt Java runtimes for exactly this purpose (the
//! official launcher uses the same ones), addressed by the `component`
//! name every version's own metadata already names in `javaVersion.component`
//! (see `core::versions::version_json`), so nothing here is tied to any
//! particular Minecraft or Java version.

use std::fmt;

pub mod detect;
pub mod install;
pub mod manifest;
pub mod runtime;

#[derive(Debug)]
pub enum JavaInstallError {
    Network(String),
    Parse(String),
    Io(std::io::Error),
    /// This OS/architecture combination isn't one Mojang publishes a Java
    /// runtime for (e.g. Linux on ARM). Not recoverable from here - the
    /// user has to install a matching Java themselves.
    UnsupportedPlatform,
    ComponentUnavailable {
        component: String,
    },
    DownloadFailed {
        failed: usize,
        total: usize,
    },
    /// Every file downloaded and landed where expected, but running the
    /// resulting `java -version` still didn't produce something
    /// recognizable - never reported as a success on faith alone.
    VerificationFailed {
        path: std::path::PathBuf,
    },
}

impl fmt::Display for JavaInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error installing Java: {msg}"),
            Self::Parse(msg) => write!(f, "Couldn't understand Mojang's Java runtime data: {msg}"),
            Self::Io(err) => write!(f, "File error installing Java: {err}"),
            Self::UnsupportedPlatform => write!(
                f,
                "TapkaCraft can't auto-install Java on this platform - install a matching Java yourself."
            ),
            Self::ComponentUnavailable { component } => write!(
                f,
                "Mojang doesn't publish the \"{component}\" Java runtime for this platform."
            ),
            Self::DownloadFailed { failed, total } => write!(
                f,
                "{failed} of {total} Java runtime files failed to download."
            ),
            Self::VerificationFailed { path } => write!(
                f,
                "Installed Java at {} but it didn't run successfully afterward.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for JavaInstallError {}

impl From<std::io::Error> for JavaInstallError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl serde::Serialize for JavaInstallError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
