use eframe::egui::{self, Color32, Frame, Margin, ProgressBar, RichText, Rounding, Stroke, Vec2};
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
// Material 3 Expressive Design Tokens & Accessible Dark Palette
// High contrast, expressive surfaces, and authentic Material You hierarchy
// ==============================================================================
const COLOR_BG: Color32 = Color32::from_rgb(16, 17, 21); // #101115 Deep Background
const COLOR_SURFACE: Color32 = Color32::from_rgb(23, 24, 30); // #17181E Surface Container Low
const COLOR_CONTAINER: Color32 = Color32::from_rgb(30, 31, 39); // #1E1F27 Surface Container
const COLOR_CONTAINER_HIGH: Color32 = Color32::from_rgb(41, 42, 53); // #292A35 Surface Container High
#[allow(dead_code)]
const COLOR_CONTAINER_HIGHEST: Color32 = Color32::from_rgb(52, 54, 67); // #343643

// Accents & Typography (Tonal Palette)
const COLOR_PRIMARY: Color32 = Color32::from_rgb(208, 188, 255); // #D0BCFF Soft Lavender / Violet
const COLOR_ON_PRIMARY: Color32 = Color32::from_rgb(56, 30, 114); // #381E72 Deep Violet (Text inside primary button)
const COLOR_PRIMARY_CONTAINER: Color32 = Color32::from_rgb(79, 55, 139); // #4F378B Primary Tonal Fill
const COLOR_ON_PRIMARY_CONTAINER: Color32 = Color32::from_rgb(234, 221, 255); // #EADDFF
const COLOR_SECONDARY: Color32 = Color32::from_rgb(204, 194, 220); // #CCC2DC
const COLOR_TEAL: Color32 = Color32::from_rgb(128, 203, 196); // #80CBC4

// High-Contrast Readable Text Tokens
const COLOR_TEXT_HEADLINE: Color32 = Color32::from_rgb(243, 239, 245); // #F3EFF5 Crisp Off-White (100% Readable)
const COLOR_TEXT_BODY: Color32 = Color32::from_rgb(202, 196, 208); // #CAC4D0 On-Surface Variant
const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(155, 151, 162); // #9B97A2 Secondary Information

// Subtle Borders (Tonal definition without wireframes)
const COLOR_BORDER_SUBTLE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 12);
const COLOR_BORDER_FOCUS: Color32 = Color32::from_rgba_premultiplied(208, 188, 255, 60);

// Hardware LED & Thermal Profile Accents
const COLOR_QUIET: Color32 = Color32::from_rgb(100, 181, 246); // Blue LED
const COLOR_BALANCED: Color32 = Color32::from_rgb(245, 245, 247); // White LED
const COLOR_PERF: Color32 = Color32::from_rgb(255, 110, 110); // Red LED
const COLOR_CUSTOM: Color32 = Color32::from_rgb(217, 128, 250); // Purple LED
const COLOR_SUCCESS: Color32 = Color32::from_rgb(129, 199, 132); // Green
const COLOR_WARNING: Color32 = Color32::from_rgb(255, 183, 77); // Amber

// ==============================================================================
// Vector Graphics & Icon Painters (Zero Missing Glyphs / Tofu Proof)
// ==============================================================================
fn status_dot(ui: &mut egui::Ui, color: Color32, radius: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(radius * 2.0 + 4.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

fn draw_legion_badge(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let center = rect.center();
    let r = size * 0.44;
    let painter = ui.painter();

    // Squircle container base
    painter.rect_filled(rect, Rounding::same(size * 0.28), COLOR_CONTAINER_HIGH);
    painter.rect_stroke(rect, Rounding::same(size * 0.28), Stroke::new(1.0_f32, COLOR_PRIMARY));

    // Three-spoke star emblem
    for angle_deg in [-90.0f32, 30.0f32, 150.0f32] {
        let rad = angle_deg.to_radians();
        let end = center + Vec2::new(rad.cos(), rad.sin()) * (r * 0.82);
        painter.line_segment([center, end], Stroke::new(2.2_f32, COLOR_PRIMARY));
    }
    painter.circle_filled(center, r * 0.22, COLOR_BG);
    painter.circle_stroke(center, r * 0.22, Stroke::new(1.2_f32, COLOR_PRIMARY));
}

// ==============================================================================
// Telemetry State & Live Sensor Readers
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
    // 1. Scan /sys/class/hwmon for dedicated AMD Ryzen (k10temp) or Intel (coretemp) sensor
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

fn configure_system_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        "/run/current-system/sw/share/X11/fonts/NotoSansNerdFont-Regular.ttf",
        "/run/current-system/sw/share/X11/fonts/UbuntuSansNerdFont-Regular.ttf",
        "/run/current-system/sw/share/X11/fonts/UbuntuMonoNerdFont-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    ];

    for path in candidates {
        if let Ok(font_bytes) = fs::read(path) {
            fonts.font_data.insert(
                "system_nerd_font".to_owned(),
                egui::FontData::from_owned(font_bytes),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "system_nerd_font".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "system_nerd_font".to_owned());
            break;
        }
    }

    ctx.set_fonts(fonts);
}

// ==============================================================================
// Native Rust GUI Application State
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
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load system fonts for crisp symbols and Nerd Font glyphs
        configure_system_fonts(&cc.egui_ctx);

        let (tx, rx) = channel();

        // Spawn background polling thread
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

        // Configure Material 3 Dark Scheme Visuals
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(COLOR_TEXT_HEADLINE);
        visuals.panel_fill = COLOR_BG;
        visuals.window_fill = COLOR_SURFACE;
        visuals.faint_bg_color = COLOR_SURFACE;
        visuals.extreme_bg_color = COLOR_BG;

        visuals.widgets.noninteractive.bg_fill = COLOR_SURFACE;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_HEADLINE);
        visuals.widgets.noninteractive.rounding = Rounding::same(18.0);

        visuals.widgets.inactive.bg_fill = COLOR_CONTAINER;
        visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_HEADLINE);
        visuals.widgets.inactive.rounding = Rounding::same(18.0);

        visuals.widgets.hovered.bg_fill = COLOR_CONTAINER_HIGH;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER_FOCUS);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.hovered.rounding = Rounding::same(18.0);

        visuals.widgets.active.bg_fill = COLOR_PRIMARY;
        visuals.widgets.active.bg_stroke = Stroke::NONE;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, COLOR_ON_PRIMARY);
        visuals.widgets.active.rounding = Rounding::same(18.0);

        visuals.selection.bg_fill = COLOR_PRIMARY;
        visuals.selection.stroke = Stroke::new(1.0_f32, COLOR_ON_PRIMARY);

        ctx.set_visuals(visuals);

        // Top App Bar
        egui::TopBottomPanel::top("top_panel")
            .frame(
                Frame::none()
                    .fill(COLOR_SURFACE)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(24.0, 14.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Legion Star Emblem & Header Info
                    draw_legion_badge(ui, 34.0);
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let model = &self.state.dmi.product_version;
                            ui.label(
                                RichText::new(model)
                                    .size(15.0)
                                    .strong()
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.label(
                                RichText::new(" v1.5.0 ")
                                    .size(11.0)
                                    .strong()
                                    .color(COLOR_PRIMARY)
                                    .background_color(COLOR_CONTAINER_HIGH),
                            );
                        });
                        let bios = &self.state.dmi.bios_version;
                        ui.label(
                            RichText::new(format!("BIOS: {} · Native Linux Driver Active", bios))
                                .size(11.0)
                                .color(COLOR_TEXT_MUTED),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Live Sync Indicator
                        ui.horizontal(|ui| {
                            status_dot(ui, COLOR_SUCCESS, 4.0);
                            ui.label(
                                RichText::new("Live Telemetry")
                                    .size(12.0)
                                    .color(COLOR_TEXT_BODY),
                            );
                        });

                        ui.add_space(12.0);

                        // Battery Pill
                        if let Some(b) = &self.state.battery {
                            let batt_str = format!("{}% ({})", b.capacity_percent, b.status);
                            let batt_color = if b.status == "Charging" {
                                COLOR_SUCCESS
                            } else {
                                COLOR_PRIMARY
                            };
                            ui.horizontal(|ui| {
                                status_dot(ui, batt_color, 4.0);
                                ui.label(
                                    RichText::new(batt_str)
                                        .size(12.0)
                                        .color(COLOR_TEXT_HEADLINE),
                                );
                            });
                        }

                        ui.add_space(12.0);

                        // Thermal Mode Indicator
                        if let Some(t) = &self.state.thermal {
                            let color = match t.active_mode.as_str() {
                                "quiet" => COLOR_QUIET,
                                "performance" => COLOR_PERF,
                                "custom" => COLOR_CUSTOM,
                                _ => COLOR_BALANCED,
                            };
                            ui.horizontal(|ui| {
                                status_dot(ui, color, 4.0);
                                ui.label(
                                    RichText::new(&t.power_led_color)
                                        .size(12.0)
                                        .color(COLOR_TEXT_HEADLINE),
                                );
                            });
                        }
                    });
                });
            });

        // Navigation Bar Tabs (Material 3 Expressive Chips)
        egui::TopBottomPanel::top("tabs_panel")
            .frame(
                Frame::none()
                    .fill(COLOR_BG)
                    .inner_margin(Margin::symmetric(24.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let tabs = [
                        (AppTab::Dashboard, "Dashboard"),
                        (AppTab::CustomMode, "Custom Power & GPU"),
                        (AppTab::Cooling, "Cooling Fans"),
                        (AppTab::Battery, "Battery & Privacy"),
                        (AppTab::Lighting, "4-Zone RGB"),
                    ];

                    for (tab, label) in tabs {
                        let is_active = self.current_tab == tab;
                        let text = RichText::new(label)
                            .size(13.0)
                            .strong()
                            .color(if is_active {
                                COLOR_ON_PRIMARY
                            } else {
                                COLOR_TEXT_HEADLINE
                            });

                        let btn = egui::Button::new(text)
                            .fill(if is_active {
                                COLOR_PRIMARY
                            } else {
                                COLOR_CONTAINER
                            })
                            .rounding(Rounding::same(9999.0))
                            .min_size(Vec2::new(140.0, 36.0));

                        if ui.add(btn).clicked() {
                            self.current_tab = tab;
                        }
                    }
                });
            });

        // Central Content View Area
        egui::CentralPanel::default()
            .frame(Frame::none().fill(COLOR_BG).inner_margin(Margin::same(24.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match self.current_tab {
                    AppTab::Dashboard => self.view_dashboard(ui),
                    AppTab::CustomMode => self.view_custom_mode(ui),
                    AppTab::Cooling => self.view_cooling(ui),
                    AppTab::Battery => self.view_battery(ui),
                    AppTab::Lighting => self.view_lighting(ui),
                });
            });

        // Bottom Notification Banner
        if let Some((msg, time)) = &self.status_msg {
            if time.elapsed() < Duration::from_secs(3) {
                egui::TopBottomPanel::bottom("status_bar")
                    .frame(
                        Frame::none()
                            .fill(COLOR_CONTAINER_HIGH)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                            .inner_margin(Margin::symmetric(24.0, 10.0)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            status_dot(ui, COLOR_SUCCESS, 4.0);
                            ui.label(
                                RichText::new(msg)
                                    .color(COLOR_PRIMARY)
                                    .size(13.0)
                                    .strong(),
                            );
                        });
                    });
            }
        }

        // Live repaint trigger
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

// ==============================================================================
// Subview Implementations
// ==============================================================================
impl LegionSpaceApp {
    fn view_dashboard(&mut self, ui: &mut egui::Ui) {
        // Welcome Hero Banner (Material 3 Surface Container)
        Frame::none()
            .fill(COLOR_CONTAINER)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("LENOVO LEGION HARDWARE CONTROL SUITE")
                            .size(11.0)
                            .color(COLOR_PRIMARY)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Welcome to Legion Space")
                            .size(24.0)
                            .strong()
                            .color(COLOR_TEXT_HEADLINE),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(
                            "Material 3 native systems manager for AMD Ryzen & NVIDIA RTX hardware",
                        )
                        .size(13.0)
                        .color(COLOR_TEXT_BODY),
                    );
                });
            });

        ui.add_space(18.0);

        // Thermal Profile Selector Section
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(20.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Thermal Power Profile")
                        .size(17.0)
                        .strong()
                        .color(COLOR_TEXT_HEADLINE),
                );
                ui.label(
                    RichText::new(
                        "Direct ACPI platform profile switching with physical power button LED sync",
                    )
                    .size(12.0)
                    .color(COLOR_TEXT_BODY),
                );

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
                        (
                            "balanced",
                            "Balanced",
                            "Adaptive Auto Dynamics",
                            COLOR_BALANCED,
                        ),
                        (
                            "performance",
                            "Performance",
                            "High Clocks · Accelerated",
                            COLOR_PERF,
                        ),
                        ("custom", "Custom", "Manual Power Envelopes", COLOR_CUSTOM),
                    ];

                    for (idx, (id, title, desc, col)) in profiles.iter().enumerate() {
                        let is_active = current_mode == *id;

                        let btn_frame = Frame::none()
                            .fill(if is_active {
                                COLOR_PRIMARY_CONTAINER
                            } else {
                                COLOR_CONTAINER
                            })
                            .stroke(if is_active {
                                Stroke::new(1.5_f32, COLOR_PRIMARY)
                            } else {
                                Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE)
                            })
                            .rounding(Rounding::same(18.0))
                            .inner_margin(Margin::symmetric(14.0, 12.0));

                        btn_frame.show(&mut cols[idx], |ui| {
                            let sense = ui.interact(
                                ui.max_rect(),
                                ui.make_persistent_id(format!("prof_{}", id)),
                                egui::Sense::click(),
                            );
                            if sense.clicked() {
                                switch_to_profile = Some(*id);
                            }

                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    status_dot(ui, *col, 4.0);
                                    ui.label(
                                        RichText::new(*title)
                                            .size(14.0)
                                            .strong()
                                            .color(if is_active {
                                                COLOR_ON_PRIMARY_CONTAINER
                                            } else {
                                                COLOR_TEXT_HEADLINE
                                            }),
                                    );
                                });
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(*desc)
                                        .size(11.0)
                                        .color(COLOR_TEXT_MUTED),
                                );
                            });
                        });
                    }
                });

                if let Some(target_profile) = switch_to_profile {
                    if let Ok(updated) = thermal::set_thermal_profile(target_profile) {
                        self.notify(format!("Switched thermal profile to {}", updated.active_mode));
                    }
                }
            });

        ui.add_space(18.0);

        // Hardware Telemetry Gauges (CPU, RAM, GPU)
        ui.columns(3, |cols| {
            // CPU Telemetry Card
            Frame::none()
                .fill(COLOR_SURFACE)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                .rounding(Rounding::same(20.0))
                .inner_margin(Margin::same(20.0))
                .show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("CPU")
                                .size(15.0)
                                .strong()
                                .color(COLOR_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let val_str = format!("{:.0}%", self.state.telemetry.cpu_usage_pct);
                            ui.label(
                                RichText::new(val_str)
                                    .size(22.0)
                                    .strong()
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    let cpu_ratio = (self.state.telemetry.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
                    ui.add(
                        ProgressBar::new(cpu_ratio)
                            .desired_height(6.0)
                            .rounding(Rounding::same(4.0))
                            .fill(COLOR_PRIMARY),
                    );

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Frequency").size(12.0).color(COLOR_TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{:.2} GHz",
                                    self.state.telemetry.cpu_freq_ghz
                                ))
                                .size(12.0)
                                .strong()
                                .color(COLOR_TEXT_BODY),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Temperature").size(12.0).color(COLOR_TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let temp = self.state.telemetry.cpu_temp_c;
                            let temp_col = if temp > 85 {
                                COLOR_PERF
                            } else if temp > 70 {
                                COLOR_WARNING
                            } else {
                                COLOR_SUCCESS
                            };
                            ui.label(
                                RichText::new(format!("{} °C", temp))
                                    .size(12.0)
                                    .strong()
                                    .color(temp_col),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Fan 1 (CPU)").size(12.0).color(COLOR_TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let fan1_rpm = self
                                .state
                                .fans
                                .as_ref()
                                .and_then(|f| f.fans.first())
                                .map(|f| f.current_rpm)
                                .unwrap_or(0);
                            ui.label(
                                RichText::new(format!("{} RPM", fan1_rpm))
                                    .size(12.0)
                                    .strong()
                                    .color(COLOR_TEXT_BODY),
                            );
                        });
                    });
                });

            // RAM Telemetry Card
            Frame::none()
                .fill(COLOR_SURFACE)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                .rounding(Rounding::same(20.0))
                .inner_margin(Margin::same(20.0))
                .show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("RAM")
                                .size(15.0)
                                .strong()
                                .color(COLOR_SECONDARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let val_str = format!("{:.0}%", self.state.telemetry.ram_usage_pct);
                            ui.label(
                                RichText::new(val_str)
                                    .size(22.0)
                                    .strong()
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    let ram_ratio = (self.state.telemetry.ram_usage_pct / 100.0).clamp(0.0, 1.0);
                    ui.add(
                        ProgressBar::new(ram_ratio)
                            .desired_height(6.0)
                            .rounding(Rounding::same(4.0))
                            .fill(COLOR_SECONDARY),
                    );

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Memory Used").size(12.0).color(COLOR_TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{:.1} GB",
                                    self.state.telemetry.ram_used_gb
                                ))
                                .size(12.0)
                                .strong()
                                .color(COLOR_TEXT_BODY),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Total RAM").size(12.0).color(COLOR_TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{:.1} GB",
                                    self.state.telemetry.ram_total_gb
                                ))
                                .size(12.0)
                                .strong()
                                .color(COLOR_TEXT_BODY),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Memory Type").size(12.0).color(COLOR_TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("DDR5 Dual-Channel")
                                    .size(12.0)
                                    .strong()
                                    .color(COLOR_TEXT_BODY),
                            );
                        });
                    });
                });

            // GPU Telemetry Card
            Frame::none()
                .fill(COLOR_SURFACE)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                .rounding(Rounding::same(20.0))
                .inner_margin(Margin::same(20.0))
                .show(&mut cols[2], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("GPU")
                                .size(15.0)
                                .strong()
                                .color(COLOR_TEAL),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let temp = self.state.gpu.as_ref().map(|g| g.current_temp_c).unwrap_or(0);
                            let temp_col = if temp > 80 {
                                COLOR_PERF
                            } else if temp > 65 {
                                COLOR_WARNING
                            } else {
                                COLOR_SUCCESS
                            };
                            ui.label(
                                RichText::new(format!("{} °C", temp))
                                    .size(22.0)
                                    .strong()
                                    .color(temp_col),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    let gpu_power = self.state.gpu.as_ref().map(|g| g.power_draw_w).unwrap_or(0.0);
                    let gpu_ratio = (gpu_power / 100.0).clamp(0.0, 1.0);
                    ui.add(
                        ProgressBar::new(gpu_ratio)
                            .desired_height(6.0)
                            .rounding(Rounding::same(4.0))
                            .fill(COLOR_TEAL),
                    );

                    ui.add_space(12.0);
                    if let Some(g) = &self.state.gpu {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Graphics Clock").size(12.0).color(COLOR_TEXT_MUTED));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{} MHz", g.graphics_clock_mhz))
                                        .size(12.0)
                                        .strong()
                                        .color(COLOR_TEXT_BODY),
                                );
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Power Draw").size(12.0).color(COLOR_TEXT_MUTED));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{:.1} W", g.power_draw_w))
                                        .size(12.0)
                                        .strong()
                                        .color(COLOR_TEXT_BODY),
                                );
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Fan 2 (GPU)").size(12.0).color(COLOR_TEXT_MUTED));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let fan2_rpm = self
                                    .state
                                    .fans
                                    .as_ref()
                                    .and_then(|f| f.fans.get(1))
                                    .map(|f| f.current_rpm)
                                    .unwrap_or(0);
                                ui.label(
                                    RichText::new(format!("{} RPM", fan2_rpm))
                                        .size(12.0)
                                        .strong()
                                        .color(COLOR_TEXT_BODY),
                                );
                            });
                        });
                    }
                });
        });

        ui.add_space(18.0);

        // Quick Access Controls Section
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(20.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Quick Hardware Controls")
                        .size(17.0)
                        .strong()
                        .color(COLOR_TEXT_HEADLINE),
                );
                ui.label(
                    RichText::new("Instant switches for battery chemistry, USB ports, and privacy")
                        .size(12.0)
                        .color(COLOR_TEXT_BODY),
                );

                ui.add_space(14.0);

                ui.columns(3, |cols| {
                    // Quick Control 1: Conservation Mode
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(16.0))
                        .show(&mut cols[0], |ui| {
                            ui.label(RichText::new("Battery Conservation").strong().color(COLOR_TEXT_HEADLINE));
                            ui.label(RichText::new("Cap charge at 80% for longevity").size(11.0).color(COLOR_TEXT_MUTED));
                            ui.add_space(10.0);

                            let mut cons = self.state.battery.as_ref().map(|b| b.conservation_mode).unwrap_or(false);
                            let cons_label = if cons { "Enabled (80% Cap)" } else { "Disabled (100%)" };
                            if ui.checkbox(&mut cons, cons_label).changed() {
                                if battery::set_conservation_mode(cons).is_ok() {
                                    self.notify(format!("Battery Conservation: {}", if cons { "Enabled" } else { "Disabled" }));
                                }
                            }
                        });

                    // Quick Control 2: Always-On USB
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(16.0))
                        .show(&mut cols[1], |ui| {
                            ui.label(RichText::new("Always-On USB").strong().color(COLOR_TEXT_HEADLINE));
                            ui.label(RichText::new("Charge devices while sleeping").size(11.0).color(COLOR_TEXT_MUTED));
                            ui.add_space(10.0);

                            let mut usb = self.state.battery.as_ref().and_then(|b| b.usb_charging).unwrap_or(false);
                            let usb_label = if usb { "Enabled (Charging)" } else { "Disabled (Off)" };
                            if ui.checkbox(&mut usb, usb_label).changed() {
                                if battery::set_usb_charging(usb).is_ok() {
                                    self.notify(format!("Always-On USB: {}", if usb { "Enabled" } else { "Disabled" }));
                                }
                            }
                        });

                    // Quick Control 3: Camera E-Shutter Privacy Switch
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(16.0))
                        .show(&mut cols[2], |ui| {
                            ui.label(RichText::new("Camera E-Shutter").strong().color(COLOR_TEXT_HEADLINE));
                            ui.label(RichText::new("Hardware privacy shutter switch").size(11.0).color(COLOR_TEXT_MUTED));
                            ui.add_space(10.0);

                            let cam_active = self.state.platform.camera_power.unwrap_or(false);
                            ui.horizontal(|ui| {
                                status_dot(ui, if cam_active { COLOR_SUCCESS } else { COLOR_WARNING }, 4.0);
                                ui.label(
                                    RichText::new(if cam_active {
                                        "Camera Active"
                                    } else {
                                        "Privacy Blocked"
                                    })
                                    .size(12.0)
                                    .strong()
                                    .color(COLOR_TEXT_HEADLINE),
                                );
                            });
                            ui.add_space(6.0);
                            let toggle_text = if cam_active { "Kill Camera" } else { "Enable Camera" };
                            if ui.button(RichText::new(toggle_text).size(11.0)).clicked() {
                                if platform::set_camera_power(!cam_active).is_ok() {
                                    self.notify(format!("Camera privacy set to {}", if !cam_active { "Enabled" } else { "Disabled" }));
                                }
                            }
                        });
                });
            });
    }

    fn view_custom_mode(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Custom Mode Hardware Envelopes & Power Tuning")
                        .size(19.0)
                        .strong()
                        .color(COLOR_TEXT_HEADLINE),
                );
                ui.label(
                    RichText::new("Fine-tune CPU sustained/boost package limits and NVIDIA RTX TGP power limits")
                        .size(12.0)
                        .color(COLOR_TEXT_BODY),
                );

                ui.add_space(20.0);

                ui.columns(2, |cols| {
                    // CPU Power Limits Column
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(20.0))
                        .show(&mut cols[0], |ui| {
                            ui.label(
                                RichText::new("CPU Power & Thermal Limits (AMD Ryzen)")
                                    .size(15.0)
                                    .strong()
                                    .color(COLOR_PRIMARY),
                            );
                            ui.add_space(14.0);

                            ui.label(
                                RichText::new(format!("PL1 (Sustained Power Limit): {} W", self.slider_pl1))
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.add(egui::Slider::new(&mut self.slider_pl1, 50..=85).suffix(" W"));

                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(format!("PL2 (Slow Package Tracking): {} W", self.slider_pl2))
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.add(egui::Slider::new(&mut self.slider_pl2, 60..=130).suffix(" W"));

                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(format!("PL3 (Fast Peak Power): {} W", self.slider_pl3))
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.add(egui::Slider::new(&mut self.slider_pl3, 70..=150).suffix(" W"));

                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(format!("CPU Thermal Throttle Ceiling: {} °C", self.slider_cpu_temp))
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.add(egui::Slider::new(&mut self.slider_cpu_temp, 85..=100).suffix(" °C"));
                        });

                    // GPU Power Limits Column
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(20.0))
                        .show(&mut cols[1], |ui| {
                            ui.label(
                                RichText::new("NVIDIA RTX 5060 Power & Thermals")
                                    .size(15.0)
                                    .strong()
                                    .color(COLOR_TEAL),
                            );
                            ui.add_space(14.0);

                            ui.label(
                                RichText::new(format!("GPU Target TGP Limit: {:.0} W", self.slider_gpu_pl))
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.add(egui::Slider::new(&mut self.slider_gpu_pl, 5.0..=115.0).suffix(" W"));

                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(format!("GPU AC Processing Baseline Offset: {} W", self.slider_ac_offset))
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.add(egui::Slider::new(&mut self.slider_ac_offset, 10..=85).suffix(" W"));

                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(format!("GPU Thermal Throttle Ceiling: {} °C", self.slider_gpu_temp))
                                    .color(COLOR_TEXT_HEADLINE),
                            );
                            ui.add(egui::Slider::new(&mut self.slider_gpu_temp, 75..=87).suffix(" °C"));

                            ui.add_space(14.0);
                            ui.label(
                                RichText::new("cTGP: 80 W (Fixed) · Dynamic Boost (PPAB): 15 W")
                                    .size(11.0)
                                    .color(COLOR_TEXT_MUTED),
                            );
                        });
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    let apply_btn = egui::Button::new(
                        RichText::new("Apply Custom Settings")
                            .strong()
                            .color(COLOR_ON_PRIMARY),
                    )
                    .fill(COLOR_PRIMARY)
                    .rounding(Rounding::same(9999.0))
                    .min_size(Vec2::new(190.0, 38.0));

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

                    let reset_btn = egui::Button::new(
                        RichText::new("Reset to Balanced Defaults")
                            .color(COLOR_TEXT_HEADLINE),
                    )
                    .fill(COLOR_CONTAINER_HIGH)
                    .rounding(Rounding::same(9999.0))
                    .min_size(Vec2::new(170.0, 38.0));

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
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Cooling Fans & Thermal Curves")
                        .size(19.0)
                        .strong()
                        .color(COLOR_TEXT_HEADLINE),
                );
                ui.label(
                    RichText::new("Independent dual blower telemetry, firmware curves, and target RPM management")
                        .size(12.0)
                        .color(COLOR_TEXT_BODY),
                );

                ui.add_space(18.0);

                // Quick Preset Pills
                ui.horizontal(|ui| {
                    let auto_btn = egui::Button::new(
                        RichText::new("Auto Curve").strong().color(COLOR_TEXT_HEADLINE),
                    )
                    .fill(COLOR_CONTAINER)
                    .rounding(Rounding::same(9999.0))
                    .min_size(Vec2::new(130.0, 36.0));
                    if ui.add(auto_btn).clicked() {
                        if fan::set_auto().is_ok() {
                            self.notify("Restored automatic firmware fan curve");
                        }
                    }

                    let full_btn = egui::Button::new(
                        RichText::new("Full Turbo (5100 RPM)").strong().color(COLOR_PERF),
                    )
                    .fill(COLOR_CONTAINER)
                    .rounding(Rounding::same(9999.0))
                    .min_size(Vec2::new(170.0, 36.0));
                    if ui.add(full_btn).clicked() {
                        if fan::set_full_speed().is_ok() {
                            self.notify("Fans set to maximum turbo speed (5100 RPM)");
                        }
                    }

                    let quiet_btn = egui::Button::new(
                        RichText::new("Silent Baseline (1700 RPM)").strong().color(COLOR_QUIET),
                    )
                    .fill(COLOR_CONTAINER)
                    .rounding(Rounding::same(9999.0))
                    .min_size(Vec2::new(180.0, 36.0));
                    if ui.add(quiet_btn).clicked() {
                        let _ = fan::set_fan_target(1, 1700);
                        let _ = fan::set_fan_target(2, 1700);
                        self.notify("Fans set to silent baseline (1700 RPM)");
                    }
                });

                ui.add_space(20.0);

                // Manual RPM Target Slider
                ui.label(
                    RichText::new(format!("Manual Target Speed: {} RPM", self.slider_fan_rpm))
                        .size(14.0)
                        .strong()
                        .color(COLOR_TEXT_HEADLINE),
                );
                let slider = egui::Slider::new(&mut self.slider_fan_rpm, 1700..=5100).suffix(" RPM");
                if ui.add(slider).changed() {
                    let _ = fan::set_fan_target(1, self.slider_fan_rpm);
                    let _ = fan::set_fan_target(2, self.slider_fan_rpm);
                }

                ui.add_space(20.0);

                // Dual Blower Live Telemetry Cards
                ui.columns(2, |cols| {
                    let fan1_rpm = self.state.fans.as_ref().and_then(|f| f.fans.first()).map(|f| f.current_rpm).unwrap_or(0);
                    let fan2_rpm = self.state.fans.as_ref().and_then(|f| f.fans.get(1)).map(|f| f.current_rpm).unwrap_or(0);

                    // Fan 1 (CPU)
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(20.0))
                        .show(&mut cols[0], |ui| {
                            ui.label(RichText::new("Fan 1 (CPU Cooler)").size(15.0).strong().color(COLOR_PRIMARY));
                            ui.add_space(6.0);
                            ui.label(RichText::new(format!("{} RPM", fan1_rpm)).size(24.0).strong().color(COLOR_TEXT_HEADLINE));
                            ui.add_space(4.0);
                            let ratio = ((fan1_rpm as f32 - 1700.0) / (5100.0 - 1700.0)).clamp(0.0, 1.0);
                            ui.add(ProgressBar::new(ratio).desired_height(6.0).rounding(Rounding::same(4.0)).fill(COLOR_PRIMARY));
                            ui.add_space(10.0);
                            ui.label(RichText::new("Operating Range: 1700 - 5100 RPM").size(11.0).color(COLOR_TEXT_MUTED));
                        });

                    // Fan 2 (GPU)
                    Frame::none()
                        .fill(COLOR_CONTAINER)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(Margin::same(20.0))
                        .show(&mut cols[1], |ui| {
                            ui.label(RichText::new("Fan 2 (GPU Cooler)").size(15.0).strong().color(COLOR_TEAL));
                            ui.add_space(6.0);
                            ui.label(RichText::new(format!("{} RPM", fan2_rpm)).size(24.0).strong().color(COLOR_TEXT_HEADLINE));
                            ui.add_space(4.0);
                            let ratio = ((fan2_rpm as f32 - 1700.0) / (5100.0 - 1700.0)).clamp(0.0, 1.0);
                            ui.add(ProgressBar::new(ratio).desired_height(6.0).rounding(Rounding::same(4.0)).fill(COLOR_TEAL));
                            ui.add_space(10.0);
                            ui.label(RichText::new("Operating Range: 1700 - 5100 RPM").size(11.0).color(COLOR_TEXT_MUTED));
                        });
                });
            });
    }

    fn view_battery(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Battery Care & Hardware Privacy")
                        .size(19.0)
                        .strong()
                        .color(COLOR_TEXT_HEADLINE),
                );
                ui.label(
                    RichText::new("Conservation threshold limiter, power telemetry, and physical e-shutter detection")
                        .size(12.0)
                        .color(COLOR_TEXT_BODY),
                );

                ui.add_space(20.0);

                let battery_info = self.state.battery.clone();
                if let Some(b) = battery_info {
                    ui.columns(2, |cols| {
                        // Column 1: Battery Telemetry & Capacity
                        Frame::none()
                            .fill(COLOR_CONTAINER)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                            .rounding(Rounding::same(18.0))
                            .inner_margin(Margin::same(20.0))
                            .show(&mut cols[0], |ui| {
                                ui.label(RichText::new("Battery Health & Delivery").size(15.0).strong().color(COLOR_PRIMARY));
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!("{}% ({})", b.capacity_percent, b.status))
                                        .size(24.0)
                                        .strong()
                                        .color(COLOR_TEXT_HEADLINE),
                                );
                                ui.add_space(4.0);
                                let cap_ratio = (b.capacity_percent as f32 / 100.0).clamp(0.0, 1.0);
                                ui.add(ProgressBar::new(cap_ratio).desired_height(6.0).rounding(Rounding::same(4.0)).fill(COLOR_PRIMARY));

                                ui.add_space(14.0);
                                if let Some(h) = b.health_percent {
                                    ui.label(RichText::new(format!("Health Rating: {:.1}%", h)).size(13.0).strong().color(COLOR_TEXT_BODY));
                                    ui.label(RichText::new(format!("Capacity: {:.1} Wh / {:.1} Wh (Design)", b.energy_full_wh, b.energy_design_wh)).size(11.0).color(COLOR_TEXT_MUTED));
                                }
                                ui.add_space(6.0);
                                ui.label(RichText::new(format!("Charge Cycles Elapsed: {}", b.cycle_count.unwrap_or(266))).size(12.0).color(COLOR_TEXT_BODY));
                                ui.label(RichText::new(format!("Power Draw Rate: {:.2} W", b.power_now_w)).size(12.0).color(COLOR_TEXT_BODY));
                            });

                        // Column 2: Switches and Privacy Controls
                        Frame::none()
                            .fill(COLOR_CONTAINER)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
                            .rounding(Rounding::same(18.0))
                            .inner_margin(Margin::same(20.0))
                            .show(&mut cols[1], |ui| {
                                ui.label(RichText::new("Hardware Control Switches").size(15.0).strong().color(COLOR_SECONDARY));
                                ui.add_space(12.0);

                                let mut conservation = b.conservation_mode;
                                if ui.checkbox(&mut conservation, "Battery Conservation (Cap at 80%)").changed() {
                                    if battery::set_conservation_mode(conservation).is_ok() {
                                        self.notify(format!("Conservation mode: {}", if conservation { "Enabled" } else { "Disabled" }));
                                    }
                                }
                                ui.label(RichText::new("Prolongs battery life when connected to AC").size(11.0).color(COLOR_TEXT_MUTED));

                                ui.add_space(14.0);

                                let mut usb = b.usb_charging.unwrap_or(false);
                                if ui.checkbox(&mut usb, "Always-On USB Charging").changed() {
                                    if battery::set_usb_charging(usb).is_ok() {
                                        self.notify(format!("Always-On USB: {}", if usb { "Enabled" } else { "Disabled" }));
                                    }
                                }
                                ui.label(RichText::new("Allows charging mobile devices while laptop sleeps").size(11.0).color(COLOR_TEXT_MUTED));

                                ui.add_space(16.0);

                                // Camera Hardware E-Shutter Section
                                let cam_active = self.state.platform.camera_power.unwrap_or(false);
                                ui.horizontal(|ui| {
                                    status_dot(ui, if cam_active { COLOR_SUCCESS } else { COLOR_WARNING }, 4.0);
                                    ui.label(
                                        RichText::new(if cam_active {
                                            "Camera E-Shutter: Enabled (Open)"
                                        } else {
                                            "Camera E-Shutter: Hardware Privacy Active"
                                        })
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_HEADLINE),
                                    );
                                });
                                ui.add_space(4.0);
                                ui.label(RichText::new("Physical slider switch on right chassis cuts power to webcam").size(11.0).color(COLOR_TEXT_MUTED));
                                ui.add_space(8.0);
                                let toggle_text = if cam_active { "Kill Camera (Disable)" } else { "Re-enable Camera" };
                                if ui.button(RichText::new(toggle_text).strong()).clicked() {
                                    if platform::set_camera_power(!cam_active).is_ok() {
                                        self.notify(format!("Camera privacy set to {}", if !cam_active { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                    });
                }
            });
    }

    fn view_lighting(&mut self, ui: &mut egui::Ui) {
        Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE))
            .rounding(Rounding::same(24.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("4-Zone RGB Keyboard Lighting")
                        .size(19.0)
                        .strong()
                        .color(COLOR_TEXT_HEADLINE),
                );
                ui.label(
                    RichText::new("Direct ITE 8258 USB HID (048d:c195) 0xCC report protocol controller")
                        .size(12.0)
                        .color(COLOR_TEXT_BODY),
                );

                ui.add_space(20.0);

                // Quick Lighting Presets
                ui.horizontal(|ui| {
                    let presets = [
                        ("legion-blue", "Legion Blue", Color32::from_rgb(0, 211, 254)),
                        ("cyberpunk", "Cyberpunk", Color32::from_rgb(255, 0, 127)),
                        ("crimson", "Crimson", Color32::from_rgb(255, 42, 42)),
                        ("spectrum", "Spectrum Wave", Color32::from_rgb(208, 188, 255)),
                        ("stealth", "Stealth (Off)", Color32::GRAY),
                    ];

                    for (id, name, col) in presets {
                        let btn = egui::Button::new(RichText::new(name).size(12.0).strong().color(col))
                            .fill(COLOR_CONTAINER)
                            .rounding(Rounding::same(9999.0))
                            .min_size(Vec2::new(130.0, 36.0));
                        if ui.add(btn).clicked() {
                            if lighting::apply_preset(id).is_ok() {
                                self.notify(format!("Applied RGB preset: {}", name));
                            }
                        }
                    }
                });

                ui.add_space(22.0);

                // Visual Keyboard 4-Zone Representation
                ui.label(RichText::new("Zone Configuration").size(14.0).strong().color(COLOR_TEXT_HEADLINE));
                ui.add_space(8.0);
                ui.columns(4, |cols| {
                    let zone_labels = [
                        ("Zone 1", "WASD / Left"),
                        ("Zone 2", "Center-Left"),
                        ("Zone 3", "Center-Right"),
                        ("Zone 4", "Numpad / Right"),
                    ];
                    for (i, (zn, desc)) in zone_labels.iter().enumerate() {
                        let is_sel = self.rgb_zone_target == (i as u8 + 1) || self.rgb_zone_target == 0;
                        Frame::none()
                            .fill(if is_sel { COLOR_CONTAINER_HIGH } else { COLOR_CONTAINER })
                            .stroke(if is_sel { Stroke::new(1.5_f32, COLOR_PRIMARY) } else { Stroke::new(1.0_f32, COLOR_BORDER_SUBTLE) })
                            .rounding(Rounding::same(14.0))
                            .inner_margin(Margin::same(14.0))
                            .show(&mut cols[i], |ui| {
                                ui.label(RichText::new(*zn).size(13.0).strong().color(COLOR_TEXT_HEADLINE));
                                ui.label(RichText::new(*desc).size(11.0).color(COLOR_TEXT_MUTED));
                            });
                    }
                });

                ui.add_space(20.0);

                // Target Zone Selector Pills
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Target Zone:").strong().color(COLOR_TEXT_HEADLINE));
                    let zones = [(0, "All Zones"), (1, "Zone 1"), (2, "Zone 2"), (3, "Zone 3"), (4, "Zone 4")];
                    for (z, label) in zones {
                        let is_active = self.rgb_zone_target == z;
                        let btn = egui::Button::new(
                            RichText::new(label)
                                .strong()
                                .color(if is_active { COLOR_ON_PRIMARY } else { COLOR_TEXT_HEADLINE }),
                        )
                        .fill(if is_active { COLOR_PRIMARY } else { COLOR_CONTAINER })
                        .rounding(Rounding::same(9999.0));
                        if ui.add(btn).clicked() {
                            self.rgb_zone_target = z;
                        }
                    }
                });

                ui.add_space(16.0);

                // Color Picker & Brightness Slider
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Color:").strong().color(COLOR_TEXT_HEADLINE));
                    ui.color_edit_button_srgb(&mut self.rgb_color);

                    ui.add_space(12.0);

                    let apply_color_btn = egui::Button::new(
                        RichText::new("Apply Solid Color").strong().color(COLOR_ON_PRIMARY),
                    )
                    .fill(COLOR_PRIMARY)
                    .rounding(Rounding::same(9999.0))
                    .min_size(Vec2::new(150.0, 32.0));

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

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Brightness:").strong().color(COLOR_TEXT_HEADLINE));
                    ui.add(egui::Slider::new(&mut self.rgb_brightness, 0..=3).suffix(" / 3"));
                });
            });
    }
}

// ==============================================================================
// Application Entry Point
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
