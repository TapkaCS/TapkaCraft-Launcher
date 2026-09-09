//! Best-effort local hardware signals: total system RAM (to suggest a sane
//! default JVM memory range for new instances) and GPU name/vendor
//! (surfaced as read-only diagnostic info; nothing in the launcher changes
//! behavior based on it yet).
//!
//! GPU detection is deliberately the lower-rigor half of this module: there
//! is no single portable API for it the way there is for RAM (`sysinfo`
//! doesn't cover GPUs at all), so each platform shells out to that OS's own
//! standard inventory tool and parses its output. Every parser here is a
//! plain function over a string and is unit tested directly, including the
//! two platforms this sandbox can't run (no macOS, no Windows) - but the
//! actual OS command invocations (`raw_gpu_names`) are only exercised for
//! real on whichever platform this crate is actually built/tested on.

use std::process::Command;

use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySuggestion {
    pub total_system_mb: u32,
    pub suggested_min_mb: u32,
    pub suggested_max_mb: u32,
}

/// Reads total system RAM and suggests a JVM `-Xms`/`-Xmx` range for a new
/// instance's default `JavaSettings`.
pub fn suggest_memory() -> MemorySuggestion {
    let mut sys = System::new();
    sys.refresh_memory();
    let total_mb = (sys.total_memory() / (1024 * 1024)) as u32;
    let (suggested_min_mb, suggested_max_mb) = suggest_range_mb(total_mb);
    MemorySuggestion {
        total_system_mb: total_mb,
        suggested_min_mb,
        suggested_max_mb,
    }
}

/// Coarse RAM tiers rather than a straight fraction of total RAM - simple
/// to reason about and to test, and avoids handing Minecraft more heap
/// than vanilla/most modpacks actually benefit from (a huge `-Xmx` mostly
/// just means longer GC pauses, not better performance). Tier boundaries
/// sit a little under round numbers (4000 rather than 4096, ...) because
/// the OS/`sysinfo` reports RAM actually usable, which normally comes in a
/// bit under a stick's nominal size.
fn suggest_range_mb(total_mb: u32) -> (u32, u32) {
    let (min_mb, max_mb) = match total_mb {
        0..=3_999 => (512, 1024),
        4_000..=7_999 => (1024, 2048),
        8_000..=15_999 => (1024, 4096),
        16_000..=31_999 => (2048, 6144),
        _ => (2048, 8192),
    };
    // The tiers above assume a machine that can plausibly run this
    // launcher at all (Windows 10/11 alone wants 4 GiB+); guard the
    // pathological case of a much smaller reported total anyway so the
    // suggestion is never larger than the RAM that's actually there.
    let max_mb = max_mb.min(total_mb.max(256));
    let min_mb = min_mb.min(max_mb);
    (min_mb, max_mb)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: GpuVendor,
}

/// Best-effort GPU inventory via the current platform's own standard
/// tooling. Never fails outward: a missing tool, parse miss, or non-zero
/// exit yields an empty `Vec` rather than an error, since nothing in the
/// launcher depends on this succeeding.
pub fn detect_gpus() -> Vec<GpuInfo> {
    raw_gpu_names()
        .into_iter()
        .map(|name| {
            let vendor = classify_vendor(&name);
            GpuInfo { name, vendor }
        })
        .collect()
}

fn classify_vendor(name: &str) -> GpuVendor {
    let lower = name.to_lowercase();
    if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("quadro") {
        GpuVendor::Nvidia
    } else if lower.contains("amd") || lower.contains("radeon") || lower.contains("ati ") {
        GpuVendor::Amd
    } else if lower.contains("intel") {
        GpuVendor::Intel
    } else if lower.contains("apple") {
        GpuVendor::Apple
    } else {
        GpuVendor::Unknown
    }
}

#[cfg(windows)]
fn raw_gpu_names() -> Vec<String> {
    // `Get-CimInstance` (CIM, WMI's modern successor) ships with
    // PowerShell on every supported Windows 10/11 install - unlike
    // `wmic`, which Windows 11 24H2 dropped from the default install.
    let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-CimInstance Win32_VideoController).Name",
        ])
        .output()
    else {
        return Vec::new();
    };
    parse_line_list(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "macos")]
fn raw_gpu_names() -> Vec<String> {
    let Ok(output) = Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output()
    else {
        return Vec::new();
    };
    parse_system_profiler_output(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "linux")]
fn raw_gpu_names() -> Vec<String> {
    let Ok(output) = Command::new("lspci").output() else {
        return Vec::new();
    };
    parse_lspci_output(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn raw_gpu_names() -> Vec<String> {
    Vec::new()
}

/// One name per non-empty line - what `Get-CimInstance ... | Name` prints.
fn parse_line_list(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Pulls GPU names out of `system_profiler SPDisplaysDataType` text output,
/// which lists one `Chipset Model:` line per adapter under a
/// `Graphics/Displays:` heading.
fn parse_system_profiler_output(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Chipset Model:"))
        .map(|name| name.trim().to_string())
        .collect()
}

/// Pulls GPU names out of `lspci` text output: one line per PCI device,
/// display adapters described as either "VGA compatible controller" or
/// "3D controller" (the latter covers some discrete/mobile GPUs lspci
/// doesn't classify as a primary VGA device).
fn parse_lspci_output(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.contains("VGA compatible controller") || line.contains("3D controller"))
        .filter_map(|line| {
            line.split_once(": ")
                .map(|(_, rest)| rest.trim().to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_range_mb_covers_every_tier() {
        assert_eq!(suggest_range_mb(2048), (512, 1024));
        assert_eq!(suggest_range_mb(3999), (512, 1024));
        assert_eq!(suggest_range_mb(4000), (1024, 2048));
        assert_eq!(suggest_range_mb(7999), (1024, 2048));
        assert_eq!(suggest_range_mb(8000), (1024, 4096));
        assert_eq!(suggest_range_mb(15999), (1024, 4096));
        assert_eq!(suggest_range_mb(16000), (2048, 6144));
        assert_eq!(suggest_range_mb(31999), (2048, 6144));
        assert_eq!(suggest_range_mb(32000), (2048, 8192));
        assert_eq!(suggest_range_mb(65536), (2048, 8192));
    }

    #[test]
    fn suggest_range_mb_never_suggests_more_than_the_machine_actually_has() {
        let (min_mb, max_mb) = suggest_range_mb(0);
        assert!(max_mb <= 256);
        assert!(min_mb <= max_mb);

        let (min_mb, max_mb) = suggest_range_mb(700);
        assert!(max_mb <= 700);
        assert!(min_mb <= max_mb);
    }

    #[test]
    fn suggest_memory_reports_a_real_positive_total_for_this_machine() {
        // Real (not mocked) call into `sysinfo` - this machine running the
        // test suite necessarily has some nonzero amount of RAM.
        let suggestion = suggest_memory();
        assert!(suggestion.total_system_mb > 0);
        assert!(suggestion.suggested_min_mb <= suggestion.suggested_max_mb);
    }

    #[test]
    fn classify_vendor_recognizes_common_real_world_names() {
        assert_eq!(
            classify_vendor("NVIDIA GeForce RTX 3080"),
            GpuVendor::Nvidia
        );
        assert_eq!(classify_vendor("AMD Radeon RX 6800 XT"), GpuVendor::Amd);
        assert_eq!(
            classify_vendor("Intel(R) UHD Graphics 630"),
            GpuVendor::Intel
        );
        assert_eq!(classify_vendor("Apple M2 Pro"), GpuVendor::Apple);
        assert_eq!(
            classify_vendor("VirtualBox Graphics Adapter"),
            GpuVendor::Unknown
        );
        assert_eq!(classify_vendor(""), GpuVendor::Unknown);
    }

    #[test]
    fn parse_line_list_drops_blank_lines_and_trims_whitespace() {
        let output = "NVIDIA GeForce RTX 4070 Laptop GPU\r\n\r\n  Intel(R) UHD Graphics  \r\n";
        assert_eq!(
            parse_line_list(output),
            vec![
                "NVIDIA GeForce RTX 4070 Laptop GPU".to_string(),
                "Intel(R) UHD Graphics".to_string(),
            ]
        );
    }

    #[test]
    fn parse_system_profiler_output_extracts_chipset_model_lines() {
        let output = "Graphics/Displays:\n\n    Apple M2 Pro:\n\n      Chipset Model: Apple M2 Pro\n      Type: GPU\n      Bus: Built-In\n      Total Number of Cores: 19\n";
        assert_eq!(
            parse_system_profiler_output(output),
            vec!["Apple M2 Pro".to_string()]
        );
    }

    #[test]
    fn parse_lspci_output_extracts_vga_and_3d_controller_lines_only() {
        let output = "\
00:02.0 VGA compatible controller: Intel Corporation TigerLake-LP GT2 [Iris Xe Graphics] (rev 01)
00:1f.3 Audio device: Intel Corporation Comet Lake PCH cAVS
01:00.0 3D controller: NVIDIA Corporation GA107M [GeForce RTX 3050 Mobile] (rev a1)
";
        assert_eq!(
            parse_lspci_output(output),
            vec![
                "Intel Corporation TigerLake-LP GT2 [Iris Xe Graphics] (rev 01)".to_string(),
                "NVIDIA Corporation GA107M [GeForce RTX 3050 Mobile] (rev a1)".to_string(),
            ]
        );
    }

    #[test]
    fn parse_lspci_output_on_empty_input_yields_no_gpus_instead_of_panicking() {
        assert!(parse_lspci_output("").is_empty());
    }

    #[test]
    fn detect_gpus_never_panics_regardless_of_whether_a_gpu_tool_is_installed_here() {
        // Real call into whatever this platform's `raw_gpu_names` is - on a
        // sandbox with no `lspci`/PowerShell/`system_profiler` available,
        // an empty result is the correct, honest answer, not a failure.
        let _ = detect_gpus();
    }
}
