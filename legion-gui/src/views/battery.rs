use eframe::egui::{self, Frame, Margin, RichText, Rounding, Vec2};
use legion_cli::hardware::{battery, platform};

use crate::icons::{draw_icon_battery, draw_icon_camera, draw_icon_power};
use crate::telemetry::AppStateSnapshot;
use crate::theme::*;
use crate::widgets::{m3_progress_bar, m3_ring_gauge, m3_switch};

pub fn render_battery(
    ui: &mut egui::Ui,
    state: &AppStateSnapshot,
    notify: &mut impl FnMut(String),
) {
    Frame::none()
        .fill(SURFACE_CONTAINER_LOW)
        .rounding(ROUNDING_SQUIRCLE_XL)
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                draw_icon_battery(ui.painter(), icon_rect, PRIMARY, 85.0);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Battery Care & Hardware Privacy").size(20.0).strong().color(ON_SURFACE));
                    ui.label(RichText::new("Conservation threshold limiter, power delivery metrics, and physical e-shutter").size(12.0).color(OUTLINE_MUTED));
                });
            });

            ui.add_space(20.0);

            if let Some(b) = state.battery.clone() {
                ui.columns(2, |cols| {
                    // Column 1: Battery Telemetry & Capacity
                    Frame::none()
                        .fill(SURFACE_CONTAINER)
                        .rounding(Rounding::same(22.0))
                        .inner_margin(Margin::same(20.0))
                        .show(&mut cols[0], |ui| {
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                                draw_icon_battery(ui.painter(), icon_rect, PRIMARY, b.capacity_percent as f32);
                                ui.label(RichText::new("Battery Health & Power Delivery").size(15.0).strong().color(PRIMARY));
                            });
                            ui.add_space(14.0);

                            let cap_ratio = (b.capacity_percent as f32 / 100.0).clamp(0.0, 1.0);
                            let val_str = format!("{}%", b.capacity_percent);
                            ui.vertical_centered(|ui| {
                                m3_ring_gauge(ui, cap_ratio, &val_str, &b.status, PRIMARY, 46.0);
                            });

                            ui.add_space(14.0);
                            m3_progress_bar(ui, cap_ratio, 8.0, PRIMARY);

                            ui.add_space(14.0);
                            if let Some(h) = b.health_percent {
                                ui.label(RichText::new(format!("Battery Health Rating: {:.1}%", h)).size(13.0).strong().color(ON_SURFACE));
                                ui.label(RichText::new(format!("Capacity: {:.1} Wh / {:.1} Wh (Design)", b.energy_full_wh, b.energy_design_wh)).size(11.0).color(OUTLINE_MUTED));
                            }
                            ui.add_space(6.0);
                            ui.label(RichText::new(format!("Charge Cycles Elapsed: {}", b.cycle_count.unwrap_or(266))).size(12.0).color(ON_SURFACE_VARIANT));
                            ui.label(RichText::new(format!("Power Draw Rate: {:.2} W", b.power_now_w)).size(12.0).color(ON_SURFACE_VARIANT));
                        });

                    // Column 2: Switches and Privacy Controls
                    Frame::none()
                        .fill(SURFACE_CONTAINER)
                        .rounding(Rounding::same(22.0))
                        .inner_margin(Margin::same(20.0))
                        .show(&mut cols[1], |ui| {
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                                draw_icon_power(ui.painter(), icon_rect, SECONDARY);
                                ui.label(RichText::new("Hardware Control Switches").size(15.0).strong().color(SECONDARY));
                            });
                            ui.add_space(16.0);

                            // Conservation Switch
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                                draw_icon_battery(ui.painter(), icon_rect, PRIMARY, 80.0);
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Battery Conservation (80%)").strong().color(ON_SURFACE));
                                    ui.label(RichText::new("Prolongs battery longevity when on AC").size(11.0).color(OUTLINE_MUTED));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let mut cons = b.conservation_mode;
                                    if m3_switch(ui, &mut cons) {
                                        if battery::set_conservation_mode(cons).is_ok() {
                                            notify(format!("Conservation: {}", if cons { "Enabled (80%)" } else { "Disabled" }));
                                        }
                                    }
                                });
                            });

                            ui.add_space(18.0);

                            // Always-On USB Switch
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                                draw_icon_power(ui.painter(), icon_rect, SECONDARY);
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Always-On USB Charging").strong().color(ON_SURFACE));
                                    ui.label(RichText::new("Charge peripherals while suspended").size(11.0).color(OUTLINE_MUTED));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let mut usb = b.usb_charging.unwrap_or(false);
                                    if m3_switch(ui, &mut usb) {
                                        if battery::set_usb_charging(usb).is_ok() {
                                            notify(format!("Always-On USB: {}", if usb { "Enabled" } else { "Disabled" }));
                                        }
                                    }
                                });
                            });

                            ui.add_space(18.0);

                            // Camera Hardware Switch
                            let cam_on = state.platform.camera_power.unwrap_or(false);
                            ui.horizontal(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                                draw_icon_camera(ui.painter(), icon_rect, if cam_on { COLOR_SUCCESS } else { COLOR_PERF }, cam_on);
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Camera Hardware E-Shutter").strong().color(ON_SURFACE));
                                    let status_str = if cam_on { "Active (Open)" } else { "Hardware Shutter Killed" };
                                    ui.label(RichText::new(status_str).size(11.0).color(if cam_on { COLOR_SUCCESS } else { COLOR_PERF }));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let mut cam = state.platform.camera_power.unwrap_or(false);
                                    if m3_switch(ui, &mut cam) {
                                        if platform::set_camera_power(cam).is_ok() {
                                            notify(format!("Camera power: {}", if cam { "Enabled" } else { "Killed" }));
                                        }
                                    }
                                });
                            });
                        });
                });
            }
        });
}

