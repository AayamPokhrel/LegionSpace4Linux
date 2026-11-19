use legion_cli::hardware::{
    battery::{self, BatteryDetails},
    custom_mode::{self, CustomModeConfig},
    fan::{self, FanSystemStatus},
    gpu::{self, GpuTelemetry},
    lighting::{self, LightingStatus},
    platform::{self, PlatformFeatures},
    probe::{self, DmiInfo},
    thermal::{self, ThermalStatus},
};
use std::fs;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

#[derive(Clone, Default)]
pub struct SystemTelemetry {
    pub cpu_usage_pct: f32,
    pub cpu_freq_ghz: f32,
    pub cpu_temp_c: u32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub ram_usage_pct: f32,
}

#[derive(Clone)]
pub struct AppStateSnapshot {
    pub dmi: DmiInfo,
    pub thermal: Option<ThermalStatus>,
    pub fans: Option<FanSystemStatus>,
    pub gpu: Option<GpuTelemetry>,
    pub battery: Option<BatteryDetails>,
    pub platform: PlatformFeatures,
    #[allow(dead_code)]
    pub lighting: Option<LightingStatus>,
    #[allow(dead_code)]
    pub custom: Option<CustomModeConfig>,
    pub telemetry: SystemTelemetry,
}

struct CpuSample {
    idle: f64,
    total: f64,
}

fn read_cpu_sample() -> Option<CpuSample> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let line = stat.lines().next()?;
    let fields: Vec<f64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    if fields.len() >= 4 {
        let idle = fields[3] + fields.get(4).copied().unwrap_or(0.0);
        let total: f64 = fields.iter().sum();
        Some(CpuSample { idle, total })
    } else {
        None
    }
}

pub fn read_cpu_freq_ghz() -> f32 {
    if let Ok(info) = fs::read_to_string("/proc/cpuinfo") {
        let mut freqs = Vec::new();
        for line in info.lines() {
            if line.starts_with("cpu MHz") {
                if let Some(val_str) = line.split(':').nth(1) {
                    if let Ok(mhz) = val_str.trim().parse::<f32>() {
                        freqs.push(mhz);
                    }
                }
            }
        }
        if !freqs.is_empty() {
            return (freqs.iter().sum::<f32>() / freqs.len() as f32) / 1000.0;
        }
    }
    3.0
}

pub fn read_ram_stats() -> (f32, f32, f32) {
    if let Ok(info) = fs::read_to_string("/proc/meminfo") {
        let mut total_kb = 0.0;
        let mut avail_kb = 0.0;
        for line in info.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
            } else if line.starts_with("MemAvailable:") {
                avail_kb = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
            }
        }
        let total_gb = total_kb / (1024.0 * 1024.0);
        let used_gb = (total_kb - avail_kb) / (1024.0 * 1024.0);
        let pct = if total_kb > 0.0 {
            (used_gb / total_gb) * 100.0
        } else {
            0.0
        };
        return (used_gb, total_gb, pct);
    }
    (8.0, 32.0, 25.0)
}

pub fn read_cpu_temp_c() -> u32 {
    // 1. Scan /sys/class/hwmon for dedicated AMD Ryzen (k10temp) or Intel sensor
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(name) = fs::read_to_string(p.join("name")) {
                let name = name.trim();
                if name == "k10temp" || name == "coretemp" || name == "zenpower" {
                    for temp_file in ["temp1_input", "temp2_input", "temp3_input"] {
                        if let Ok(val_str) = fs::read_to_string(p.join(temp_file)) {
                            if let Ok(mut val) = val_str.trim().parse::<u32>() {
                                if val > 1000 {
                                    val /= 1000;
                                }
                                if (20..=115).contains(&val) {
                                    return val;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Generic hwmon fallback scan
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(val_str) = fs::read_to_string(p.join("temp1_input")) {
                if let Ok(mut val) = val_str.trim().parse::<u32>() {
                    if val > 1000 {
                        val /= 1000;
                    }
                    if (25..=105).contains(&val) {
                        return val;
                    }
                }
            }
        }
    }
    45
}

pub fn start_telemetry_thread() -> (Receiver<AppStateSnapshot>, AppStateSnapshot) {
    let (tx, rx) = channel();

    let initial = AppStateSnapshot {
        dmi: probe::probe_dmi(),
        thermal: thermal::get_thermal_status().ok(),
        fans: fan::get_fan_status().ok(),
        gpu: gpu::get_gpu_telemetry().ok(),
        battery: battery::get_battery_details().ok(),
        platform: platform::get_platform_features(),
        lighting: lighting::get_lighting_status().ok(),
        custom: custom_mode::get_custom_mode_config().ok(),
        telemetry: SystemTelemetry {
            cpu_usage_pct: 0.0,
            cpu_freq_ghz: read_cpu_freq_ghz(),
            cpu_temp_c: read_cpu_temp_c(),
            ram_used_gb: 8.0,
            ram_total_gb: 32.0,
            ram_usage_pct: 25.0,
        },
    };

    thread::spawn(move || {
        let mut prev_sample = read_cpu_sample();
        let mut cpu_pct = 0.0f32;

        loop {
            thread::sleep(Duration::from_millis(850));

            if let Some(curr) = read_cpu_sample() {
                if let Some(prev) = prev_sample {
                    let idle_delta = curr.idle - prev.idle;
                    let total_delta = curr.total - prev.total;
                    if total_delta > 0.0 {
                        cpu_pct = (1.0 - (idle_delta / total_delta)) as f32 * 100.0;
                        cpu_pct = cpu_pct.clamp(0.0, 100.0);
                    }
                }
                prev_sample = Some(curr);
            }

            let cpu_freq = read_cpu_freq_ghz();
            let cpu_temp = read_cpu_temp_c();
            let (ram_used, ram_total, ram_pct) = read_ram_stats();

            let dmi = probe::probe_dmi();
            let thermal = thermal::get_thermal_status().ok();
            let fans = fan::get_fan_status().ok();
            let gpu = gpu::get_gpu_telemetry().ok();
            let battery = battery::get_battery_details().ok();
            let platform = platform::get_platform_features();
            let lighting = lighting::get_lighting_status().ok();
            let custom = custom_mode::get_custom_mode_config().ok();

            let snapshot = AppStateSnapshot {
                dmi,
                thermal,
                fans,
                gpu,
                battery,
                platform,
                lighting,
                custom,
                telemetry: SystemTelemetry {
                    cpu_usage_pct: cpu_pct,
                    cpu_freq_ghz: cpu_freq,
                    cpu_temp_c: cpu_temp,
                    ram_used_gb: ram_used,
                    ram_total_gb: ram_total,
                    ram_usage_pct: ram_pct,
                },
            };

            let _ = tx.send(snapshot);
        }
    });

    (rx, initial)
}

pub fn configure_system_fonts(ctx: &eframe::egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();
    let candidates = [
        "/run/current-system/sw/share/X11/fonts/GoogleSansCodeNerdFontPropo-Regular.ttf",
        "/run/current-system/sw/share/X11/fonts/NotoSansNerdFont-Regular.ttf",
        "/run/current-system/sw/share/X11/fonts/UbuntuSansNerdFont-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    ];

    for path in candidates {
        if let Ok(font_bytes) = fs::read(path) {
            fonts.font_data.insert(
                "google_sans".to_owned(),
                eframe::egui::FontData::from_owned(font_bytes),
            );
            fonts
                .families
                .entry(eframe::egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "google_sans".to_owned());
            fonts
                .families
                .entry(eframe::egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "google_sans".to_owned());
            break;
        }
    }

    ctx.set_fonts(fonts);
}
