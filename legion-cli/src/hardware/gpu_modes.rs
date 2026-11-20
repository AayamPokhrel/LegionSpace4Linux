use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuWorkingMode {
    Hybrid,
    #[serde(rename = "dgpu")]
    DGpuOnly,
    #[serde(rename = "igpu")]
    IGpuOnly,
    #[serde(rename = "auto")]
    HybridAuto,
}

impl std::fmt::Display for GpuWorkingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuWorkingMode::Hybrid => write!(f, "Hybrid (Optimus)"),
            GpuWorkingMode::DGpuOnly => write!(f, "dGPU Mode (Discrete)"),
            GpuWorkingMode::IGpuOnly => write!(f, "Hybrid-iGPU Mode (Eco)"),
            GpuWorkingMode::HybridAuto => write!(f, "Hybrid-Auto Mode"),
        }
    }
}

impl std::str::FromStr for GpuWorkingMode {
    type Err = LegionError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().trim() {
            "hybrid" | "optimus" => Ok(GpuWorkingMode::Hybrid),
            "dgpu" | "discrete" | "dedicated" => Ok(GpuWorkingMode::DGpuOnly),
            "igpu" | "integrated" | "eco" | "battery" => Ok(GpuWorkingMode::IGpuOnly),
            "auto" | "hybrid-auto" | "hybrid_auto" => Ok(GpuWorkingMode::HybridAuto),
            _ => Err(LegionError::InvalidValue(format!(
                "Invalid GPU mode '{}'. Valid modes: hybrid, dgpu, igpu, auto",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProcessInfo {
    pub pid: u32,
    pub name: String,
    pub used_memory_mib: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuModeStatus {
    pub active_mode: GpuWorkingMode,
    pub mux_status: String,
    pub dgpu_power_state: String,
    pub powermizer_mode: Option<u32>,
    pub running_processes: Vec<GpuProcessInfo>,
    pub ac_connected: bool,
}

const GPU_MODE_CONFIG_PATH: &str = "/home/aayam422h/.config/legionspace/gpu_mode.json";

pub fn is_ac_connected() -> bool {
    let adp_paths = [
        "/sys/class/power_supply/ADP0/online",
        "/sys/class/power_supply/ADP1/online",
        "/sys/class/power_supply/AC/online",
    ];
    for p in adp_paths {
        if let Ok(val) = fs::read_to_string(p) {
            if val.trim() == "1" {
                return true;
            }
        }
    }
    true
}

pub fn get_saved_gpu_mode() -> GpuWorkingMode {
    if let Ok(data) = fs::read_to_string(GPU_MODE_CONFIG_PATH) {
        if let Ok(mode) = serde_json::from_str::<GpuWorkingMode>(&data) {
            return mode;
        }
    }
    GpuWorkingMode::Hybrid
}

pub fn save_gpu_mode(mode: GpuWorkingMode) {
    if let Some(parent) = Path::new(GPU_MODE_CONFIG_PATH).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        GPU_MODE_CONFIG_PATH,
        serde_json::to_string_pretty(&mode).unwrap_or_default(),
    );
}

pub fn get_gpu_runtime_pm_state() -> String {
    let p = "/sys/bus/pci/devices/0000:01:00.0/power/runtime_status";
    if let Ok(s) = fs::read_to_string(p) {
        return s.trim().to_string();
    }
    "active".to_string()
}

pub fn get_mux_status() -> String {
    // Check if internal panel is currently connected to NVIDIA dGPU (eDP-2) or AMD iGPU (eDP-1)
    let nvidia_edp = "/sys/class/drm/card1-eDP-2/status";
    let amd_edp = "/sys/class/drm/card2-eDP-1/status";

    if let Ok(s) = fs::read_to_string(nvidia_edp) {
        if s.trim() == "connected" {
            return "Discrete dGPU (Parade PS8371 MUX switched to RTX 5060)".to_string();
        }
    }
    if let Ok(s) = fs::read_to_string(amd_edp) {
        if s.trim() == "connected" {
            return "Dynamic Display Switch (AMD Raphael iGPU driving panel)".to_string();
        }
    }
    "Advanced Optimus (Dynamic DDS)".to_string()
}

pub fn get_gpu_processes() -> Vec<GpuProcessInfo> {
    let mut procs = Vec::new();
    let output = Command::new("nvidia-smi")
        .args([
            "--query-compute-apps=pid,process_name,used_gpu_memory",
            "--format=csv,noheader,nounits",
        ])
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                if let (Ok(pid), Ok(mem)) = (parts[0].parse::<u32>(), parts[2].parse::<u64>()) {
                    procs.push(GpuProcessInfo {
                        pid,
                        name: parts[1].to_string(),
                        used_memory_mib: mem,
                    });
                }
            }
        }
    }
    procs
}

pub fn get_powermizer_mode() -> Option<u32> {
    let output = Command::new("nvidia-settings")
        .env("DISPLAY", ":0")
        .args(["-q", "GPUPowerMizerMode"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.contains("Attribute 'GPUPowerMizerMode'") {
            if let Some(pos) = line.find("): ") {
                let val_str = &line[pos + 3..].trim_end_matches('.');
                if let Ok(val) = val_str.trim().parse::<u32>() {
                    return Some(val);
                }
            }
        }
    }
    Some(0) // Default Adaptive
}

pub fn set_powermizer_mode(mode: u32) -> Result<()> {
    let status = Command::new("nvidia-settings")
        .env("DISPLAY", ":0")
        .args(["-a", &format!("[gpu:0]/GPUPowerMizerMode={}", mode)])
        .status()
        .map_err(LegionError::Io)?;

    if status.success() {
        Ok(())
    } else {
        Err(LegionError::UnsupportedFeature(
            "Failed to set GPUPowerMizerMode via nvidia-settings".into(),
        ))
    }
}

pub fn get_gpu_mode_status() -> GpuModeStatus {
    let active_mode = get_saved_gpu_mode();
    let mux_status = get_mux_status();
    let dgpu_power_state = get_gpu_runtime_pm_state();
    let powermizer_mode = get_powermizer_mode();
    let running_processes = get_gpu_processes();
    let ac_connected = is_ac_connected();

    GpuModeStatus {
        active_mode,
        mux_status,
        dgpu_power_state,
        powermizer_mode,
        running_processes,
        ac_connected,
    }
}

pub fn set_gpu_working_mode(mode: GpuWorkingMode) -> Result<GpuModeStatus> {
    match mode {
        GpuWorkingMode::Hybrid => {
            // Enable PCI runtime PM auto for dynamic on-demand usage
            let _ = fs::write("/sys/bus/pci/devices/0000:01:00.0/power/control", "auto");
            let _ = set_powermizer_mode(0); // Adaptive
        }
        GpuWorkingMode::DGpuOnly => {
            // Force dGPU power on
            let _ = fs::write("/sys/bus/pci/devices/0000:01:00.0/power/control", "on");
            let _ = set_powermizer_mode(1); // Prefer Maximum Performance
        }
        GpuWorkingMode::IGpuOnly => {
            // Suspend NVIDIA GPU into D3cold: set power control to auto
            let _ = fs::write("/sys/bus/pci/devices/0000:01:00.0/power/control", "auto");
            let _ = fs::write("/sys/bus/pci/devices/0000:01:00.0/d3cold_allowed", "1");
            let _ = set_powermizer_mode(0); // Adaptive
        }
        GpuWorkingMode::HybridAuto => {
            let ac = is_ac_connected();
            if ac {
                let _ = fs::write("/sys/bus/pci/devices/0000:01:00.0/power/control", "auto");
                let _ = set_powermizer_mode(0);
            } else {
                let _ = fs::write("/sys/bus/pci/devices/0000:01:00.0/power/control", "auto");
                let _ = fs::write("/sys/bus/pci/devices/0000:01:00.0/d3cold_allowed", "1");
            }
        }
    }

    save_gpu_mode(mode);
    Ok(get_gpu_mode_status())
}
