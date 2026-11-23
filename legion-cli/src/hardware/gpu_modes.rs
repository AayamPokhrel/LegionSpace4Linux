use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfigFile {
    pub active_mode: GpuWorkingMode,
    #[serde(default)]
    pub powermizer_mode: Option<u32>,
}

pub fn get_config_dir() -> PathBuf {
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        if !sudo_user.is_empty() && sudo_user != "root" {
            let user_home = PathBuf::from(format!("/home/{}", sudo_user));
            if user_home.exists() {
                return user_home.join(".config").join("legionspace");
            }
        }
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg);
        if p.is_absolute() {
            return p.join("legionspace");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config").join("legionspace");
    }
    PathBuf::from("/etc/legionspace")
}

#[cfg(unix)]
pub fn fix_file_owner<P: AsRef<Path>>(path: P) {
    use std::os::unix::ffi::OsStrExt;
    if let (Ok(uid_s), Ok(gid_s)) = (std::env::var("SUDO_UID"), std::env::var("SUDO_GID")) {
        if let (Ok(uid), Ok(gid)) = (uid_s.parse::<u32>(), gid_s.parse::<u32>()) {
            if let Ok(c_path) = std::ffi::CString::new(path.as_ref().as_os_str().as_bytes()) {
                unsafe {
                    libc::chown(c_path.as_ptr(), uid, gid);
                }
            }
        }
    }
}

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

pub fn load_gpu_config() -> GpuConfigFile {
    let path = get_config_dir().join("gpu_mode.json");
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<GpuConfigFile>(&data) {
            return cfg;
        }
        if let Ok(mode) = serde_json::from_str::<GpuWorkingMode>(&data) {
            return GpuConfigFile {
                active_mode: mode,
                powermizer_mode: Some(0),
            };
        }
    }
    GpuConfigFile {
        active_mode: GpuWorkingMode::Hybrid,
        powermizer_mode: Some(0),
    }
}

pub fn save_gpu_config(cfg: &GpuConfigFile) {
    let dir = get_config_dir();
    let _ = fs::create_dir_all(&dir);
    fix_file_owner(&dir);
    let path = dir.join("gpu_mode.json");
    if let Ok(data) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(&path, data);
        fix_file_owner(&path);
    }
}

pub fn get_saved_gpu_mode() -> GpuWorkingMode {
    load_gpu_config().active_mode
}

pub fn save_gpu_mode(mode: GpuWorkingMode) {
    let mut cfg = load_gpu_config();
    cfg.active_mode = mode;
    save_gpu_config(&cfg);
}

pub fn get_saved_powermizer_mode() -> u32 {
    load_gpu_config().powermizer_mode.unwrap_or(0)
}

pub fn save_powermizer_mode(mode: u32) {
    let mut cfg = load_gpu_config();
    cfg.powermizer_mode = Some(mode);
    save_gpu_config(&cfg);
}

fn get_nvidia_pci_dir() -> PathBuf {
    let standard = PathBuf::from("/sys/bus/pci/devices/0000:01:00.0");
    if standard.exists() {
        return standard;
    }
    if let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") {
        for entry in entries.flatten() {
            let vendor_path = entry.path().join("vendor");
            if let Ok(vendor) = fs::read_to_string(vendor_path) {
                if vendor.trim().eq_ignore_ascii_case("0x10de") {
                    return entry.path();
                }
            }
        }
    }
    standard
}

pub fn get_gpu_runtime_pm_state() -> String {
    let p = get_nvidia_pci_dir().join("power/runtime_status");
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

fn prepare_nvidia_settings_command() -> Command {
    let mut cmd = Command::new("nvidia-settings");

    if let Ok(disp) = std::env::var("DISPLAY") {
        cmd.env("DISPLAY", disp);
    } else {
        cmd.env("DISPLAY", ":0");
    }

    if let Ok(xauth) = std::env::var("XAUTHORITY") {
        cmd.env("XAUTHORITY", xauth);
    } else {
        let uid = unsafe { libc::getuid() };
        let run_user = format!("/run/user/{}", uid);
        if let Ok(entries) = fs::read_dir(&run_user) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let s = name.to_string_lossy();
                if s.starts_with(".mutter-Xwaylandauth") || s.starts_with("Xauthority") {
                    cmd.env("XAUTHORITY", entry.path());
                    break;
                }
            }
        }
    }

    cmd
}

pub fn get_powermizer_mode() -> Option<u32> {
    let mut cmd = prepare_nvidia_settings_command();
    cmd.args(["-q", "GPUPowerMizerMode"]);

    if let Ok(output) = cmd.output() {
        if output.status.success() {
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
        }
    }

    // Fall back to saved configured powermizer mode
    Some(get_saved_powermizer_mode())
}

pub fn set_powermizer_mode(mode: u32) -> Result<()> {
    // Always persist the user's selected PowerMizer mode
    save_powermizer_mode(mode);

    // Attempt to apply via nvidia-settings if driver/display is reachable
    let mut cmd = prepare_nvidia_settings_command();
    cmd.args(["-a", &format!("[gpu:0]/GPUPowerMizerMode={}", mode)]);

    let _ = cmd.status();
    Ok(())
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
    let pci = get_nvidia_pci_dir();
    let control_path = pci.join("power/control");
    let d3cold_path = pci.join("d3cold_allowed");

    match mode {
        GpuWorkingMode::Hybrid => {
            // Enable PCI runtime PM auto for dynamic on-demand usage
            let _ = fs::write(&control_path, "auto");
            let _ = fs::write(&d3cold_path, "1");
            let _ = set_powermizer_mode(0); // Adaptive
        }
        GpuWorkingMode::DGpuOnly => {
            // Force dGPU power on
            let _ = fs::write(&control_path, "on");
            let _ = fs::write(&d3cold_path, "0");
            let _ = set_powermizer_mode(1); // Prefer Maximum Performance
        }
        GpuWorkingMode::IGpuOnly => {
            // Suspend NVIDIA GPU into D3cold: set power control to auto
            let _ = fs::write(&control_path, "auto");
            let _ = fs::write(&d3cold_path, "1");
            let _ = set_powermizer_mode(0); // Adaptive
        }
        GpuWorkingMode::HybridAuto => {
            let ac = is_ac_connected();
            if ac {
                let _ = fs::write(&control_path, "auto");
                let _ = fs::write(&d3cold_path, "1");
                let _ = set_powermizer_mode(0);
            } else {
                let _ = fs::write(&control_path, "auto");
                let _ = fs::write(&d3cold_path, "1");
            }
        }
    }

    save_gpu_mode(mode);
    Ok(get_gpu_mode_status())
}
