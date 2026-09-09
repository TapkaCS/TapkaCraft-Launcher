//! `DownloadManager`: the one place network fetches for game files,
//! libraries, assets and (later) mod/modpack content go through - the UI
//! layer never issues fetches directly. Concurrency-limited, retried,
//! SHA1-verified, and atomic (temp file + rename) so a crash or cancel
//! can never leave a half-written file mistaken for a real one.

use std::fmt;

pub mod manager;
pub mod task;

pub use manager::{download_all, DownloadSummary};
pub use task::DownloadTask;

#[derive(Debug)]
pub enum DownloadError {
    Network(String),
    Io(std::io::Error),
    HashMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    Other(String),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error: {msg}"),
            Self::Io(err) => write!(f, "File error: {err}"),
            Self::HashMismatch {
                file,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "\"{file}\" is corrupted (expected sha1 {expected}, got {actual})"
                )
            }
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DownloadError {}

impl From<std::io::Error> for DownloadError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Progress/result events for one download batch. Serializable so the
/// Tauri command layer can forward them to the frontend as-is via
/// `app.emit`; `core::downloads` itself has no Tauri dependency.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DownloadEvent {
    Started {
        total_files: usize,
    },
    FileProgress {
        label: String,
        bytes_downloaded: u64,
        total_bytes: Option<u64>,
    },
    FileCompleted {
        label: String,
        cached: bool,
    },
    FileFailed {
        label: String,
        error: String,
    },
    Finished {
        succeeded: usize,
        failed: usize,
    },
}

/// Where `DownloadEvent`s go. Wraps an optional channel sender so callers
/// that don't care about progress (most tests) can use `ProgressSink::none()`
/// instead of threading `Option` handling through every call site.
#[derive(Clone)]
pub struct ProgressSink(Option<tokio::sync::mpsc::UnboundedSender<DownloadEvent>>);

impl ProgressSink {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<DownloadEvent>) -> Self {
        Self(Some(sender))
    }

    pub fn none() -> Self {
        Self(None)
    }

    pub fn send(&self, event: DownloadEvent) {
        if let Some(sender) = &self.0 {
            let _ = sender.send(event);
        }
    }
}
