use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Vec2};
use legion_cli::hardware::{
    battery::{self, BatteryDetails},
    custom_mode::{self, CustomModeConfig},
    fan::{self, FanSystemStatus},
    gpu::{self, GpuTelemetry},
    lighting::{self, LightingEffect, LightingStatus, RgbColor},
    platform::{self, PlatformFeatures},
    probe::{self, DmiInfo},
    thermal::{self, ThermalStatus},
};
use std::fs;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::{Duration, Instant};

// ==============================================================================
// Material You (Material 3 Expressive) Color Palette & Design Tokens
// ==============================================================================
const COLOR_BG: Color32 = Color32::from_rgb(18, 19, 22);
const COLOR_SURFACE: Color32 = Color32::from_rgb(26, 27, 32);
const COLOR_CONTAINER: Color32 = Color32::from_rgb(33, 35, 43);
const COLOR_CONTAINER_HIGH: Color32 = Color32::from_rgb(41, 43, 54);
const COLOR_PRIMARY: Color32 = Color32::from_rgb(208, 188, 255); // Soft Lavender Accent
const COLOR_ON_PRIMARY: Color32 = Color32::from_rgb(56, 30, 114);
const COLOR_SECONDARY: Color32 = Color32::from_rgb(204, 194, 220);
const COLOR_TEXT_MAIN: Color32 = Color32::from_rgb(230, 225, 229);
const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(147, 143, 153);
const COLOR_BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 18);

// Power LED & Thermal Colors
const COLOR_QUIET: Color32 = Color32::from_rgb(100, 181, 246); // Blue
const COLOR_BALANCED: Color32 = Color32::from_rgb(245, 245, 245); // White
const COLOR_PERF: Color32 = Color32::from_rgb(239, 83, 80); // Red
const COLOR_CUSTOM: Color32 = Color32::from_rgb(171, 71, 188); // Purple

// ==============================================================================
// Telemetry State & CPU/RAM Reader
// ==============================================================================
#[derive(Clone, Default)]
struct SystemTelemetry {
    cpu_usage_pct: f32,
    cpu_freq_ghz: f32,
    cpu_temp_c: u32,
    ram_used_gb: f32,
    ram_total_gb: f32,
    ram_usage_pct: f32,
}

#[derive(Clone)]
struct AppStateSnapshot {
    dmi: DmiInfo,
    thermal: Option<ThermalStatus>,
    fans: Option<FanSystemStatus>,
    gpu: Option<GpuTelemetry>,
    battery: Option<BatteryDetails>,
    platform: PlatformFeatures,
    #[allow(dead_code)]
    lighting: Option<LightingStatus>,
    #[allow(dead_code)]
    custom: Option<CustomModeConfig>,
    telemetry: SystemTelemetry,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum AppTab {
    Dashboard,
    CustomMode,
    Cooling,
    Battery,
    Lighting,
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

fn read_cpu_freq_ghz() -> f32 {
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

fn read_ram_stats() -> (f32, f32, f32) {
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

fn read_cpu_temp_c() -> u32 {
    for path in [
        "/sys/class/hwmon/hwmon4/temp1_input",
        "/sys/class/hwmon/hwmon5/temp1_input",
        "/sys/class/thermal/thermal_zone0/temp",
    ] {
        if let Ok(val_str) = fs::read_to_string(path) {
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
    45
}

// ==============================================================================
// Native Rust GUI Application
// ==============================================================================
pub struct LegionSpaceApp {
    state: AppStateSnapshot,
    rx: Receiver<AppStateSnapshot>,
    current_tab: AppTab,
    status_msg: Option<(String, Instant)>,

    // Sliders for Custom Mode tuning
    slider_pl1: i64,
    slider_pl2: i64,
    slider_pl3: i64,
    slider_cpu_temp: i64,
    slider_gpu_temp: i64,
    slider_ac_offset: i64,
    slider_gpu_pl: f32,

    // Sliders for Fans
    slider_fan_rpm: u32,

    // Lighting colors & zone targets
    rgb_zone_target: u8, // 0 = all, 1=z1, 2=z2, 3=z3, 4=z4
    rgb_color: [u8; 3],
    rgb_brightness: u8,
    #[allow(dead_code)]
    rgb_speed: u8,
}

impl LegionSpaceApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();

        // Spawn background polling thread
        thread::spawn(move || {
            let mut prev_sample = read_cpu_sample();
            let mut cpu_pct = 0.0f32;

            loop {
                thread::sleep(Duration::from_millis(900));

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

        // Initial snapshot
        let dmi = probe::probe_dmi();
        let thermal = thermal::get_thermal_status().ok();
        let fans = fan::get_fan_status().ok();
        let gpu = gpu::get_gpu_telemetry().ok();
        let battery = battery::get_battery_details().ok();
        let platform = platform::get_platform_features();
        let lighting = lighting::get_lighting_status().ok();
        let custom = custom_mode::get_custom_mode_config().ok();

        Self {
            state: AppStateSnapshot {
                dmi,
                thermal,
                fans,
                gpu,
                battery,
                platform,
                lighting,
                custom,
                telemetry: SystemTelemetry::default(),
            },
            rx,
            current_tab: AppTab::Dashboard,
            status_msg: None,
            slider_pl1: 70,
            slider_pl2: 85,
            slider_pl3: 102,
            slider_cpu_temp: 100,
            slider_gpu_temp: 87,
            slider_ac_offset: 55,
            slider_gpu_pl: 80.0,
            slider_fan_rpm: 3500,
            rgb_zone_target: 0,
            rgb_color: [0, 211, 254], // Legion Cyan
            rgb_brightness: 3,
            rgb_speed: 2,
        }
    }

    fn notify(&mut self, text: impl Into<String>) {
        self.status_msg = Some((text.into(), Instant::now()));
    }
}

impl eframe::App for LegionSpaceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain newest state from background worker
        while let Ok(new_state) = self.rx.try_recv() {
            self.state = new_state;
        }

        // Configure Material You dark aesthetic
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(COLOR_TEXT_MAIN);
        visuals.widgets.noninteractive.bg_fill = COLOR_SURFACE;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
        visuals.widgets.noninteractive.rounding = Rounding::same(16.0);
        visuals.widgets.inactive.bg_fill = COLOR_CONTAINER;
        visuals.widgets.inactive.rounding = Rounding::same(16.0);
        visuals.widgets.hovered.bg_fill = COLOR_CONTAINER_HIGH;
        visuals.widgets.hovered.rounding = Rounding::same(16.0);
        visuals.widgets.active.bg_fill = COLOR_PRIMARY;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, COLOR_ON_PRIMARY);
        visuals.widgets.active.rounding = Rounding::same(16.0);
        ctx.set_visuals(visuals);

        // Top App Bar
        egui::TopBottomPanel::top("top_panel")
            .frame(
                Frame::none()
                    .fill(COLOR_SURFACE)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .inner_margin(Margin::symmetric(24.0, 16.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Legion Star Emblem & Title
                    ui.label(RichText::new("✦").color(COLOR_PRIMARY).size(22.0).strong());
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let model = &self.state.dmi.product_version;
                            ui.label(RichText::new(model).size(15.0).strong().color(COLOR_TEXT_MAIN));
                            ui.label(
                                RichText::new("v1.4.0 (Native Rust)")
                                    .size(11.0)
                                    .color(COLOR_PRIMARY)
                                    .background_color(COLOR_CONTAINER_HIGH),
                            );
                        });
                        let bios = &self.state.dmi.bios_version;
                        ui.label(
                            RichText::new(format!("BIOS: {} · Native Linux Driver", bios))
                                .size(11.0)
                                .color(COLOR_TEXT_MUTED),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Live Sync Pill
                        ui.label(RichText::new("● Live Sync").size(12.0).color(COLOR_QUIET));

                        // Battery Pill
                        if let Some(b) = &self.state.battery {
                            let batt_str = format!("⚡ {}% ({})", b.capacity_percent, b.status);
                            ui.label(
                                RichText::new(batt_str)
                                    .size(12.0)
                                    .color(COLOR_TEXT_MAIN)
                                    .background_color(COLOR_CONTAINER),
                            );
                        }

                        // Active Mode Pill
                        if let Some(t) = &self.state.thermal {
                            let color = match t.active_mode.as_str() {
                                "quiet" => COLOR_QUIET,
                                "performance" => COLOR_PERF,
                                "custom" => COLOR_CUSTOM,
                                _ => COLOR_BALANCED,
                            };
                            let mode_text = format!("● {}", t.power_led_color);
                            ui.label(
                                RichText::new(mode_text)
                                    .size(12.0)
                                    .color(color)
                                    .background_color(COLOR_CONTAINER),
                            );
                        }
                    });
                });
            });

        // Navigation Bar Tabs
        egui::TopBottomPanel::top("tabs_panel")
            .frame(Frame::none().fill(COLOR_BG).inner_margin(Margin::symmetric(24.0, 12.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let tabs = [
                        (AppTab::Dashboard, "📊 Dashboard"),
                        (AppTab::CustomMode, "⚡ Custom Power & GPU"),
                        (AppTab::Cooling, "🌀 Cooling Fans"),
                        (AppTab::Battery, "🔋 Battery Care"),
                        (AppTab::Lighting, "🌈 4-Zone RGB"),
                    ];

                    for (tab, label) in tabs {
                        let is_active = self.current_tab == tab;
                        let text = RichText::new(label)
                            .size(13.0)
                            .strong()
                            .color(if is_active { COLOR_ON_PRIMARY } else { COLOR_TEXT_MAIN });

                        let btn = egui::Button::new(text)
                            .fill(if is_active { COLOR_PRIMARY } else { COLOR_CONTAINER })
                            .rounding(Rounding::same(9999.0))
                            .min_size(Vec2::new(130.0, 34.0));

                        if ui.add(btn).clicked() {
                            self.current_tab = tab;
                        }
                    }
                });
            });

        // Central View Area
        egui::CentralPanel::default()
            .frame(Frame::none().fill(COLOR_BG).inner_margin(Margin::same(24.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.current_tab {
                        AppTab::Dashboard => self.view_dashboard(ui),
                        AppTab::CustomMode => self.view_custom_mode(ui),
                        AppTab::Cooling => self.view_cooling(ui),
                        AppTab::Battery => self.view_battery(ui),
                        AppTab::Lighting => self.view_lighting(ui),
                    }
                });
            });

        // Bottom Notification Banner
        if let Some((msg, time)) = &self.status_msg {
            if time.elapsed() < Duration::from_secs(3) {
                egui::TopBottomPanel::bottom("status_bar")
                    .frame(
                        Frame::none()
                            .fill(COLOR_CONTAINER_HIGH)
                            .inner_margin(Margin::symmetric(24.0, 10.0)),
                    )
                    .show(ctx, |ui| {
                        ui.label(RichText::new(format!("✓ {}", msg)).color(COLOR_PRIMARY).size(13.0).strong());
                    });
            }
        }

        // Request repaint for live telemetry
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

// ==============================================================================
// GUI Subviews
// ==============================================================================
impl LegionSpaceApp {
    fn view_dashboard(&mut self, ui: &mut egui::Ui) {
        // Welcome Hero Banner
        Frame::none()
            .fill(COLOR_CONTAINER)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("LENOVO LEGION HARDWARE CONTROL SUITE")
                                .size(11.0)
                                .color(COLOR_PRIMARY)
                                .strong(),
                        );
                        ui.label(RichText::new("Welcome to Legion Space").size(22.0).strong());
                        ui.label(
                            RichText::new("Material 3 native systems manager for AMD Ryzen & NVIDIA RTX hardware")
                                .size(13.0)
                                .color(COLOR_TEXT_MUTED),
                        );
                    });
                });
            });

        ui.add_space(20.0);

        // Thermal Profile Selector Section
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(20.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Thermal Power Profile").size(16.0).strong());
                        ui.label(
                            RichText::new("Direct ACPI platform profile switching with physical power button LED sync")
                                .size(12.0)
                                .color(COLOR_TEXT_MUTED),
                        );
                    });
                });

                ui.add_space(14.0);

                let current_mode = self
                    .state
                    .thermal
                    .as_ref()
                    .map(|t| t.active_mode.clone())
                    .unwrap_or_else(|| "balanced".to_string());

                let mut switch_to_profile = None;

                ui.columns(4, |cols| {
                    let profiles = [
                        ("quiet", "Quiet", "Silent Fans · Power Saving", COLOR_QUIET),
                        ("balanced", "Balanced", "Adaptive Auto Dynamics", COLOR_BALANCED),
                        ("performance", "Performance", "High Clocks · Accelerated", COLOR_PERF),
                        ("custom", "Custom", "Manual Power Envelopes", COLOR_CUSTOM),
                    ];

                    for (idx, (id, title, desc, col)) in profiles.iter().enumerate() {
                        let is_active = current_mode == *id;
                        let btn = egui::Button::new(
                            RichText::new(format!("● {}\n{}", title, desc))
                                .size(12.0)
                                .color(if is_active { COLOR_ON_PRIMARY } else { *col }),
                        )
                        .fill(if is_active { COLOR_PRIMARY } else { COLOR_CONTAINER })
                        .rounding(Rounding::same(18.0))
                        .min_size(Vec2::new(cols[idx].available_width(), 64.0));

                        if cols[idx].add(btn).clicked() {
                            switch_to_profile = Some(*id);
                        }
                    }
                });

                if let Some(target_profile) = switch_to_profile {
                    if let Ok(updated) = thermal::set_thermal_profile(target_profile) {
                        self.notify(format!("Switched thermal profile to {}", updated.active_mode));
                    }
                }
            });

        ui.add_space(20.0);

        // Telemetry Grid: CPU, RAM, GPU
        ui.columns(3, |cols| {
            // CPU Card
            Frame::none()
                .fill(COLOR_SURFACE)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(Rounding::same(20.0))
                .inner_margin(Margin::same(18.0))
                .show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("CPU").size(14.0).strong().color(COLOR_PRIMARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let val_str = format!("{:.0}%", self.state.telemetry.cpu_usage_pct);
                            ui.label(RichText::new(val_str).size(18.0).strong());
                        });
                    });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Frequency:    {:.2} GHz", self.state.telemetry.cpu_freq_ghz))
                            .size(12.0)
                            .color(COLOR_TEXT_MUTED),
                    );
                    ui.label(
                        RichText::new(format!("Temperature:  {} °C", self.state.telemetry.cpu_temp_c))
                            .size(12.0)
                            .color(COLOR_TEXT_MUTED),
                    );
                    let fan1_rpm = self
                        .state
                        .fans
                        .as_ref()
                        .and_then(|f| f.fans.first())
                        .map(|f| f.current_rpm)
                        .unwrap_or(0);
                    ui.label(RichText::new(format!("Fan 1 (CPU):  {} RPM", fan1_rpm)).size(12.0).color(COLOR_TEXT_MUTED));
                });

            // RAM Card
            Frame::none()
                .fill(COLOR_SURFACE)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(Rounding::same(20.0))
                .inner_margin(Margin::same(18.0))
                .show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("RAM").size(14.0).strong().color(COLOR_SECONDARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let val_str = format!("{:.0}%", self.state.telemetry.ram_usage_pct);
                            ui.label(RichText::new(val_str).size(18.0).strong());
                        });
                    });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("In Use:       {:.1} GB", self.state.telemetry.ram_used_gb))
                            .size(12.0)
                            .color(COLOR_TEXT_MUTED),
                    );
                    ui.label(
                        RichText::new(format!("Total Memory: {:.1} GB", self.state.telemetry.ram_total_gb))
                            .size(12.0)
                            .color(COLOR_TEXT_MUTED),
                    );
                    ui.label(RichText::new("Type:         DDR5 System RAM").size(12.0).color(COLOR_TEXT_MUTED));
                });

            // GPU Card
            Frame::none()
                .fill(COLOR_SURFACE)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(Rounding::same(20.0))
                .inner_margin(Margin::same(18.0))
                .show(&mut cols[2], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("GPU").size(14.0).strong().color(Color32::from_rgb(128, 203, 196)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let temp = self.state.gpu.as_ref().map(|g| g.current_temp_c).unwrap_or(0);
                            ui.label(RichText::new(format!("{} °C", temp)).size(18.0).strong());
                        });
                    });
                    ui.add_space(8.0);
                    if let Some(g) = &self.state.gpu {
                        ui.label(RichText::new(format!("Clocks:       {} MHz", g.graphics_clock_mhz)).size(12.0).color(COLOR_TEXT_MUTED));
                        ui.label(RichText::new(format!("Power Draw:   {:.1} W", g.power_draw_w)).size(12.0).color(COLOR_TEXT_MUTED));
                        let fan2_rpm = self
                            .state
                            .fans
                            .as_ref()
                            .and_then(|f| f.fans.get(1))
                            .map(|f| f.current_rpm)
                            .unwrap_or(0);
                        ui.label(RichText::new(format!("Fan 2 (GPU):  {} RPM", fan2_rpm)).size(12.0).color(COLOR_TEXT_MUTED));
                    }
                });
        });
    }

    fn view_custom_mode(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Custom Mode Hardware Envelopes & Power Tuning").size(18.0).strong());
                ui.label(
                    RichText::new("Tune CPU sustained/boost package limits and NVIDIA RTX TGP power limits")
                        .size(12.0)
                        .color(COLOR_TEXT_MUTED),
                );

                ui.add_space(16.0);

                ui.columns(2, |cols| {
                    // CPU Tuning Column
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(16.0))
                        .show(&mut cols[0], |ui| {
                            ui.label(RichText::new("CPU Power & Thermal Limits").size(14.0).strong().color(COLOR_PRIMARY));
                            ui.add_space(10.0);

                            ui.label(format!("PL1 (Sustained Power Limit): {} W", self.slider_pl1));
                            ui.add(egui::Slider::new(&mut self.slider_pl1, 50..=85).suffix(" W"));

                            ui.add_space(6.0);
                            ui.label(format!("PL2 (Slow Package Tracking): {} W", self.slider_pl2));
                            ui.add(egui::Slider::new(&mut self.slider_pl2, 60..=130).suffix(" W"));

                            ui.add_space(6.0);
                            ui.label(format!("PL3 (Fast Peak Power): {} W", self.slider_pl3));
                            ui.add(egui::Slider::new(&mut self.slider_pl3, 70..=150).suffix(" W"));

                            ui.add_space(6.0);
                            ui.label(format!("CPU Thermal Throttle Ceiling: {} °C", self.slider_cpu_temp));
                            ui.add(egui::Slider::new(&mut self.slider_cpu_temp, 85..=100).suffix(" °C"));
                        });

                    // GPU Tuning Column
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(16.0))
                        .show(&mut cols[1], |ui| {
                            ui.label(RichText::new("NVIDIA RTX 5060 Power & Thermals").size(14.0).strong().color(COLOR_SECONDARY));
                            ui.add_space(10.0);

                            ui.label(format!("GPU Target TGP Limit: {:.0} W", self.slider_gpu_pl));
                            ui.add(egui::Slider::new(&mut self.slider_gpu_pl, 5.0..=115.0).suffix(" W"));

                            ui.add_space(6.0);
                            ui.label(format!("GPU AC Processing Baseline Offset: {} W", self.slider_ac_offset));
                            ui.add(egui::Slider::new(&mut self.slider_ac_offset, 10..=85).suffix(" W"));

                            ui.add_space(6.0);
                            ui.label(format!("GPU Thermal Throttle Ceiling: {} °C", self.slider_gpu_temp));
                            ui.add(egui::Slider::new(&mut self.slider_gpu_temp, 75..=87).suffix(" °C"));

                            ui.add_space(10.0);
                            ui.label(RichText::new("cTGP: 80 W (Fixed) · Dynamic Boost (PPAB): 15 W").size(11.0).color(COLOR_TEXT_MUTED));
                        });
                });

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    let apply_btn = egui::Button::new(RichText::new("Apply Custom Settings").strong().color(COLOR_ON_PRIMARY))
                        .fill(COLOR_PRIMARY)
                        .rounding(Rounding::same(9999.0))
                        .min_size(Vec2::new(180.0, 36.0));

                    if ui.add(apply_btn).clicked() {
                        let _ = gpu::set_gpu_tuning_settings(
                            Some(self.slider_gpu_pl),
                            Some(self.slider_gpu_temp),
                            Some(self.slider_ac_offset),
                            None,
                            None,
                            None,
                        );
                        let _ = custom_mode::set_firmware_attribute("ppt_pl1_spl", self.slider_pl1);
                        let _ = custom_mode::set_firmware_attribute("ppt_pl2_sppt", self.slider_pl2);
                        let _ = custom_mode::set_firmware_attribute("ppt_pl3_fppt", self.slider_pl3);
                        let _ = custom_mode::set_firmware_attribute("cpu_temp", self.slider_cpu_temp);
                        self.notify("Custom mode power limits and GPU settings updated!");
                    }

                    let reset_btn = egui::Button::new(RichText::new("Reset to Balanced Defaults"))
                        .fill(COLOR_CONTAINER_HIGH)
                        .rounding(Rounding::same(9999.0))
                        .min_size(Vec2::new(160.0, 36.0));

                    if ui.add(reset_btn).clicked() {
                        self.slider_pl1 = 70;
                        self.slider_pl2 = 85;
                        self.slider_pl3 = 102;
                        self.slider_cpu_temp = 100;
                        self.slider_gpu_temp = 87;
                        self.slider_ac_offset = 55;
                        self.slider_gpu_pl = 80.0;
                        self.notify("Reset sliders to factory defaults");
                    }
                });
            });
    }

    fn view_cooling(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Cooling Fans & Thermal Curves").size(18.0).strong());
                ui.label(
                    RichText::new("Independent dual blower monitoring and manual target RPM management")
                        .size(12.0)
                        .color(COLOR_TEXT_MUTED),
                );

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    let auto_btn = egui::Button::new("Auto Curve")
                        .fill(COLOR_CONTAINER)
                        .rounding(Rounding::same(9999.0))
                        .min_size(Vec2::new(120.0, 36.0));
                    if ui.add(auto_btn).clicked() {
                        if fan::set_auto().is_ok() {
                            self.notify("Restored automatic firmware fan curve");
                        }
                    }

                    let full_btn = egui::Button::new("Full Turbo (5100 RPM)")
                        .fill(COLOR_CONTAINER)
                        .rounding(Rounding::same(9999.0))
                        .min_size(Vec2::new(160.0, 36.0));
                    if ui.add(full_btn).clicked() {
                        if fan::set_full_speed().is_ok() {
                            self.notify("Fans set to maximum turbo speed (5100 RPM)");
                        }
                    }
                });

                ui.add_space(16.0);

                ui.label(format!("Manual Target Speed: {} RPM", self.slider_fan_rpm));
                let slider = egui::Slider::new(&mut self.slider_fan_rpm, 1700..=5100).suffix(" RPM");
                if ui.add(slider).changed() {
                    let _ = fan::set_fan_target(1, self.slider_fan_rpm);
                    let _ = fan::set_fan_target(2, self.slider_fan_rpm);
                }

                ui.add_space(20.0);

                if let Some(f) = &self.state.fans {
                    for fan in &f.fans {
                        ui.label(format!("{}: {} RPM (Range: {} - {} RPM)", fan.label, fan.current_rpm, fan.min_rpm, fan.max_rpm));
                    }
                }
            });
    }

    fn view_battery(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Battery Care & Power Telemetry").size(18.0).strong());
                ui.label(
                    RichText::new("Conservation threshold limiter and ACPI hardware switches")
                        .size(12.0)
                        .color(COLOR_TEXT_MUTED),
                );

                ui.add_space(16.0);

                let battery_info = self.state.battery.clone();
                if let Some(b) = battery_info {
                    let mut conservation = b.conservation_mode;
                    if ui.checkbox(&mut conservation, "Battery Conservation Mode (Cap at 80% to protect longevity)").changed() {
                        if battery::set_conservation_mode(conservation).is_ok() {
                            self.notify(format!("Conservation mode: {}", if conservation { "Enabled" } else { "Disabled" }));
                        }
                    }

                    ui.add_space(8.0);

                    let mut usb = b.usb_charging.unwrap_or(false);
                    if ui.checkbox(&mut usb, "Always-On USB Charging (Charge phone while sleeping)").changed() {
                        if battery::set_usb_charging(usb).is_ok() {
                            self.notify(format!("Always-On USB: {}", if usb { "Enabled" } else { "Disabled" }));
                        }
                    }

                    ui.add_space(16.0);

                    let cam_status = if self.state.platform.camera_power.unwrap_or(false) {
                        "Enabled (Shutter Open)"
                    } else {
                        "Disabled (Physical E-Shutter Active)"
                    };
                    ui.label(format!("Camera E-Shutter Hardware Switch: {}", cam_status));
                    ui.label(format!("Charge Cycles Elapsed: {}", b.cycle_count.unwrap_or(266)));
                    if let Some(h) = b.health_percent {
                        ui.label(format!("Battery Health Rating: {:.1}% ({:.1} Wh / {:.1} Wh)", h, b.energy_full_wh, b.energy_design_wh));
                    }
                }
            });
    }

    fn view_lighting(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(RichText::new("4-Zone RGB Keyboard Lighting").size(18.0).strong());
                ui.label(
                    RichText::new("Direct ITE 8258 USB HID (048d:c195) 0xCC report protocol controller")
                        .size(12.0)
                        .color(COLOR_TEXT_MUTED),
                );

                ui.add_space(16.0);

                // Presets
                ui.horizontal(|ui| {
                    let presets = [
                        ("legion-blue", "Legion Blue", Color32::from_rgb(0, 211, 254)),
                        ("cyberpunk", "Cyberpunk", Color32::from_rgb(255, 0, 127)),
                        ("crimson", "Crimson", Color32::from_rgb(255, 42, 42)),
                        ("spectrum", "Spectrum Wave", Color32::from_rgb(208, 188, 255)),
                        ("stealth", "Stealth (Off)", Color32::GRAY),
                    ];

                    for (id, name, col) in presets {
                        let btn = egui::Button::new(RichText::new(name).size(12.0).color(col))
                            .fill(COLOR_CONTAINER)
                            .rounding(Rounding::same(9999.0))
                            .min_size(Vec2::new(120.0, 34.0));
                        if ui.add(btn).clicked() {
                            if lighting::apply_preset(id).is_ok() {
                                self.notify(format!("Applied RGB preset: {}", name));
                            }
                        }
                    }
                });

                ui.add_space(20.0);

                // Zone Selector and Color Picker
                ui.horizontal(|ui| {
                    ui.label("Target Zone:");
                    let zones = [(0, "All Zones"), (1, "Zone 1"), (2, "Zone 2"), (3, "Zone 3"), (4, "Zone 4")];
                    for (z, label) in zones {
                        let is_active = self.rgb_zone_target == z;
                        let btn = egui::Button::new(RichText::new(label).color(if is_active { COLOR_ON_PRIMARY } else { COLOR_TEXT_MAIN }))
                            .fill(if is_active { COLOR_PRIMARY } else { COLOR_CONTAINER })
                            .rounding(Rounding::same(9999.0));
                        if ui.add(btn).clicked() {
                            self.rgb_zone_target = z;
                        }
                    }
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_srgb(&mut self.rgb_color);

                    let apply_color_btn = egui::Button::new("Apply Solid Color")
                        .fill(COLOR_PRIMARY)
                        .rounding(Rounding::same(9999.0));
                    if ui.add(apply_color_btn).clicked() {
                        let color = RgbColor::new(self.rgb_color[0], self.rgb_color[1], self.rgb_color[2]);
                        if self.rgb_zone_target == 0 {
                            let _ = lighting::set_all_zones(LightingEffect::Static, 2, self.rgb_brightness, color);
                        } else {
                            let _ = lighting::set_zone_color(self.rgb_zone_target, LightingEffect::Static, 2, self.rgb_brightness, color);
                            let _ = lighting::commit_rgb_profile(1, self.rgb_brightness);
                        }
                        self.notify("Custom RGB color applied to keyboard");
                    }
                });
            });
    }
}

// ==============================================================================
// Entry Point
// ==============================================================================
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Legion Space")
            .with_inner_size([1040.0, 780.0])
            .with_min_inner_size([880.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Legion Space",
        options,
        Box::new(|cc| Ok(Box::new(LegionSpaceApp::new(cc)))),
    )
}
