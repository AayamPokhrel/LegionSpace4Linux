use eframe::egui::{self, Frame, Margin, RichText, Rounding, Vec2};
use legion_cli::hardware::{battery, platform, thermal};

use crate::icons::*;
use crate::telemetry::AppStateSnapshot;
use crate::theme::*;
use crate::widgets::{m3_ring_gauge, m3_switch};

pub fn render_dashboard(
    ui: &mut egui::Ui,
    state: &AppStateSnapshot,
    notify: &mut impl FnMut(String),
) {
    // ==============================================================================
    // 1. Material 3 Expressive Hero Card (Asymmetric 32dp Squircle)
    // ==============================================================================
    Frame::none()
        .fill(SURFACE_CONTAINER)
        .rounding(ROUNDING_SQUIRCLE_XL)
        .inner_margin(Margin::same(26.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("LENOVO LEGION HARDWARE CONTROL SUITE")
                            .size(11.0)
                            .strong()
                            .color(PRIMARY),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Welcome to Legion Space")
                            .size(26.0)
                            .strong()
                            .color(ON_SURFACE),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("AMD Ryzen 7 7745HX · NVIDIA RTX 5060 Laptop · 32GB DDR5 · Linux Kernel Driver")
                            .size(13.0)
                            .color(ON_SURFACE_VARIANT),
                    );

                    ui.add_space(16.0);

                    // Glanceable Stadium Badges
                    ui.horizontal(|ui| {
                        let badges = [
                            (
                                format!("{} °C CPU", state.telemetry.cpu_temp_c),
                                PRIMARY,
                                1, // CPU temp
                            ),
                            (
                                format!(
                                    "{} °C GPU",
                                    state.gpu.as_ref().map(|g| g.current_temp_c).unwrap_or(0)
                                ),
                                COLOR_TEAL,
                                2, // GPU temp
                            ),
                            (
                                format!(
                                    "{} RPM Fans",
                                    state
                                        .fans
                                        .as_ref()
                                        .and_then(|f| f.fans.first())
                                        .map(|f| f.current_rpm)
                                        .unwrap_or(0)
                                ),
                                SECONDARY,
                                3, // Fan
                            ),
                            (
                                format!(
                                    "{}% Battery",
                                    state.battery.as_ref().map(|b| b.capacity_percent).unwrap_or(0)
                                ),
                                COLOR_SUCCESS,
                                4, // Battery
                            ),
                        ];

                        for (text, col, icon_type) in badges {
                            Frame::none()
                                .fill(SURFACE_CONTAINER_HIGH)
                                .rounding(ROUNDING_STADIUM)
                                .inner_margin(Margin::symmetric(14.0, 6.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                                        let p = ui.painter();
                                        match icon_type {
                                            1 => draw_icon_thermometer(p, icon_rect, col),
                                            2 => draw_icon_gpu(p, icon_rect, col),
                                            3 => draw_icon_fan(p, icon_rect, col),
                                            _ => draw_icon_battery(p, icon_rect, col, 77.0),
                                        }
                                        ui.add_space(2.0);
                                        ui.label(RichText::new(text).size(12.0).strong().color(col));
                                    });
                                });
                        }
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Iconic M3 8-Lobe Scalloped Badge with Legion Star
                    let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(92.0), egui::Sense::hover());
                    let p = ui.painter();
                    let center = badge_rect.center();
                    draw_scalloped_flower(p, center, 42.0, PRIMARY_CONTAINER);
                    draw_legion_star(p, center, 28.0, PRIMARY);
                });
            });
        });

    ui.add_space(20.0);

    // ==============================================================================
    // 2. Thermal Power Profile (Segmented M3 Expressive Cards)
    // ==============================================================================
    Frame::none()
        .fill(SURFACE_CONTAINER_LOW)
        .rounding(ROUNDING_SQUIRCLE_LG)
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new("Thermal Power Profile")
                    .size(17.0)
                    .strong()
                    .color(ON_SURFACE),
            );
            ui.label(
                RichText::new("Physical power button LED indicator sync via ACPI platform profile")
                    .size(12.0)
                    .color(OUTLINE_MUTED),
            );

            ui.add_space(14.0);

            let current_mode = state
                .thermal
                .as_ref()
                .map(|t| t.active_mode.clone())
                .unwrap_or_else(|| "balanced".to_string());

            let mut switch_to_profile = None;

            ui.columns(4, |cols| {
                let profiles = [
                    ("quiet", "Quiet", "Silent Fans · Eco", COLOR_QUIET, 1),
                    ("balanced", "Balanced", "Adaptive Dynamics", COLOR_BALANCED, 2),
                    ("performance", "Performance", "Turbo Accelerated", COLOR_PERF, 3),
                    ("custom", "Custom", "Manual Envelopes", COLOR_CUSTOM, 4),
                ];

                for (idx, (id, title, desc, col, icon_kind)) in profiles.iter().enumerate() {
                    let is_active = current_mode == *id;

                    let tile_frame = Frame::none()
                        .fill(if is_active {
                            PRIMARY_CONTAINER
                        } else {
                            SURFACE_CONTAINER
                        })
                        .rounding(Rounding::same(20.0))
                        .inner_margin(Margin::same(14.0));

                    tile_frame.show(&mut cols[idx], |ui| {
                        let rect = ui.max_rect();
                        let sense = ui.interact(
                            rect,
                            ui.make_persistent_id(format!("m3_prof_{}", id)),
                            egui::Sense::click(),
                        );
                        if sense.clicked() {
                            switch_to_profile = Some(*id);
                        }

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                                let p = ui.painter();
                                match icon_kind {
                                    1 => draw_icon_leaf(p, icon_rect, *col),
                                    2 => draw_icon_gauge(p, icon_rect, *col),
                                    3 => draw_icon_flame(p, icon_rect, *col),
                                    _ => draw_icon_sliders(p, icon_rect, *col),
                                }

                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(*title)
                                        .size(14.0)
                                        .strong()
                                        .color(if is_active {
                                            ON_PRIMARY_CONTAINER
                                        } else {
                                            ON_SURFACE
                                        }),
                                );

                                if is_active {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let (chk_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                                        draw_icon_check(ui.painter(), chk_rect.center(), 12.0, PRIMARY);
                                    });
                                }
                            });

                            ui.add_space(6.0);
                            ui.label(RichText::new(*desc).size(11.0).color(OUTLINE_MUTED));
                        });
                    });
                }
            });

            if let Some(target_profile) = switch_to_profile {
                if let Ok(updated) = thermal::set_thermal_profile(target_profile) {
                    notify(format!("Switched thermal profile to {}", updated.active_mode));
                }
            }
        });

    ui.add_space(20.0);

    // ==============================================================================
    // 3. Live Hardware Telemetry (Expressive Ring Gauges)
    // ==============================================================================
    ui.columns(3, |cols| {
        // CPU Card
        Frame::none()
            .fill(SURFACE_CONTAINER)
            .rounding(ROUNDING_SQUIRCLE_LG)
            .inner_margin(Margin::same(20.0))
            .show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                    draw_icon_thermometer(ui.painter(), icon_rect, PRIMARY);
                    ui.label(RichText::new("AMD Ryzen CPU").size(14.0).strong().color(PRIMARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.2} GHz", state.telemetry.cpu_freq_ghz)).size(12.0).strong().color(ON_SURFACE_VARIANT));
                    });
                });

                ui.add_space(12.0);

                // Expressive Circular Ring Meter
                let ratio = (state.telemetry.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
                let val_str = format!("{:.0}%", state.telemetry.cpu_usage_pct);
                ui.vertical_centered(|ui| {
                    m3_ring_gauge(ui, ratio, &val_str, "LOAD", PRIMARY, 44.0);
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Package Temp").size(12.0).color(OUTLINE_MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let t = state.telemetry.cpu_temp_c;
                        let col = if t > 85 { COLOR_PERF } else if t > 70 { COLOR_WARNING } else { COLOR_SUCCESS };
                        ui.label(RichText::new(format!("{} °C", t)).size(12.0).strong().color(col));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Fan 1 (CPU)").size(12.0).color(OUTLINE_MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let rpm = state.fans.as_ref().and_then(|f| f.fans.first()).map(|f| f.current_rpm).unwrap_or(0);
                        ui.label(RichText::new(format!("{} RPM", rpm)).size(12.0).strong().color(ON_SURFACE_VARIANT));
                    });
                });
            });

        // RAM Card
        Frame::none()
            .fill(SURFACE_CONTAINER)
            .rounding(ROUNDING_SQUIRCLE_LG)
            .inner_margin(Margin::same(20.0))
            .show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                    draw_icon_ram(ui.painter(), icon_rect, SECONDARY);
                    ui.label(RichText::new("System Memory").size(14.0).strong().color(SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("DDR5").size(12.0).strong().color(ON_SURFACE_VARIANT));
                    });
                });

                ui.add_space(12.0);

                let ratio = (state.telemetry.ram_usage_pct / 100.0).clamp(0.0, 1.0);
                let val_str = format!("{:.0}%", state.telemetry.ram_usage_pct);
                ui.vertical_centered(|ui| {
                    m3_ring_gauge(ui, ratio, &val_str, "USED", SECONDARY, 44.0);
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("In Use").size(12.0).color(OUTLINE_MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.1} GB", state.telemetry.ram_used_gb)).size(12.0).strong().color(ON_SURFACE_VARIANT));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Total RAM").size(12.0).color(OUTLINE_MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{:.1} GB", state.telemetry.ram_total_gb)).size(12.0).strong().color(ON_SURFACE_VARIANT));
                    });
                });
            });

        // GPU Card
        Frame::none()
            .fill(SURFACE_CONTAINER)
            .rounding(ROUNDING_SQUIRCLE_LG)
            .inner_margin(Margin::same(20.0))
            .show(&mut cols[2], |ui| {
                ui.horizontal(|ui| {
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                    draw_icon_gpu(ui.painter(), icon_rect, COLOR_TEAL);
                    ui.label(RichText::new("NVIDIA RTX 5060").size(14.0).strong().color(COLOR_TEAL));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let pwr = state.gpu.as_ref().map(|g| g.power_draw_w).unwrap_or(0.0);
                        ui.label(RichText::new(format!("{:.1} W", pwr)).size(12.0).strong().color(ON_SURFACE_VARIANT));
                    });
                });

                ui.add_space(12.0);

                let temp = state.gpu.as_ref().map(|g| g.current_temp_c).unwrap_or(0);
                let ratio = (temp as f32 / 100.0).clamp(0.0, 1.0);
                let val_str = format!("{}°C", temp);
                ui.vertical_centered(|ui| {
                    m3_ring_gauge(ui, ratio, &val_str, "TEMP", COLOR_TEAL, 44.0);
                });

                ui.add_space(12.0);
                if let Some(g) = &state.gpu {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Graphics Clock").size(12.0).color(OUTLINE_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} MHz", g.graphics_clock_mhz)).size(12.0).strong().color(ON_SURFACE_VARIANT));
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Fan 2 (GPU)").size(12.0).color(OUTLINE_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let rpm = state.fans.as_ref().and_then(|f| f.fans.get(1)).map(|f| f.current_rpm).unwrap_or(0);
                            ui.label(RichText::new(format!("{} RPM", rpm)).size(12.0).strong().color(ON_SURFACE_VARIANT));
                        });
                    });
                }
            });
    });

    ui.add_space(20.0);

    // ==============================================================================
    // 4. Quick Hardware Switches (M3 Stadium Cards)
    // ==============================================================================
    Frame::none()
        .fill(SURFACE_CONTAINER_LOW)
        .rounding(ROUNDING_SQUIRCLE_LG)
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.label(RichText::new("Quick Hardware Switches").size(17.0).strong().color(ON_SURFACE));
            ui.label(RichText::new("Instant toggle switches for battery conservation, USB ports, and privacy").size(12.0).color(OUTLINE_MUTED));

            ui.add_space(16.0);

            ui.columns(3, |cols| {
                // Control 1: Battery Conservation
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(20.0))
                    .inner_margin(Margin::same(18.0))
                    .show(&mut cols[0], |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                            draw_icon_battery(ui.painter(), icon_rect, PRIMARY, 80.0);
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Conservation").size(14.0).strong().color(ON_SURFACE));
                                ui.label(RichText::new("80% Charge Cap").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut cons = state.battery.as_ref().map(|b| b.conservation_mode).unwrap_or(false);
                                if m3_switch(ui, &mut cons) {
                                    if battery::set_conservation_mode(cons).is_ok() {
                                        notify(format!("Conservation: {}", if cons { "Enabled (80%)" } else { "Disabled" }));
                                    }
                                }
                            });
                        });
                    });

                // Control 2: Always-On USB
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(20.0))
                    .inner_margin(Margin::same(18.0))
                    .show(&mut cols[1], |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                            draw_icon_power(ui.painter(), icon_rect, SECONDARY);
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Always-On USB").size(14.0).strong().color(ON_SURFACE));
                                ui.label(RichText::new("Charge in Sleep").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut usb = state.battery.as_ref().and_then(|b| b.usb_charging).unwrap_or(false);
                                if m3_switch(ui, &mut usb) {
                                    if battery::set_usb_charging(usb).is_ok() {
                                        notify(format!("Always-On USB: {}", if usb { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                        });
                    });

                // Control 3: Camera Hardware E-Shutter
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(20.0))
                    .inner_margin(Margin::same(18.0))
                    .show(&mut cols[2], |ui| {
                        let cam_on = state.platform.camera_power.unwrap_or(false);
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                            draw_icon_camera(ui.painter(), icon_rect, if cam_on { COLOR_SUCCESS } else { COLOR_PERF }, cam_on);
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Camera Privacy").size(14.0).strong().color(ON_SURFACE));
                                let status_str = if cam_on { "Active (Open)" } else { "Shutter Killed" };
                                ui.label(RichText::new(status_str).size(11.0).color(if cam_on { COLOR_SUCCESS } else { COLOR_PERF }));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut cam_state = state.platform.camera_power.unwrap_or(false);
                                if m3_switch(ui, &mut cam_state) {
                                    if platform::set_camera_power(cam_state).is_ok() {
                                        notify(format!("Camera power: {}", if cam_state { "Enabled" } else { "Killed" }));
                                    }
                                }
                            });
                        });
                    });
            });
        });
}
