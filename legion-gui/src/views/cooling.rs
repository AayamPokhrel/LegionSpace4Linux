use eframe::egui::{self, Frame, Margin, RichText, Rounding, Vec2};
use legion_cli::hardware::fan;

use crate::icons::draw_icon_fan;
use crate::telemetry::AppStateSnapshot;
use crate::theme::*;
use crate::widgets::{m3_progress_bar, m3_ring_gauge, m3_slider_u32};

pub fn render_cooling(
    ui: &mut egui::Ui,
    state: &AppStateSnapshot,
    slider_fan_rpm: &mut u32,
    notify: &mut impl FnMut(String),
) {
    Frame::none()
        .fill(SURFACE_CONTAINER_LOW)
        .rounding(ROUNDING_SQUIRCLE_XL)
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                draw_icon_fan(ui.painter(), icon_rect, PRIMARY);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Cooling Fans & Thermal Curves").size(20.0).strong().color(ON_SURFACE));
                    ui.label(RichText::new("Dual independent blower monitoring and manual target RPM management").size(12.0).color(OUTLINE_MUTED));
                });
            });

            ui.add_space(20.0);

            // Quick Mode Chips
            ui.horizontal(|ui| {
                let auto_btn = egui::Button::new(RichText::new("Auto Curve").strong().color(ON_SURFACE))
                    .fill(SURFACE_CONTAINER)
                    .rounding(ROUNDING_STADIUM)
                    .min_size(Vec2::new(130.0, 38.0));
                if ui.add(auto_btn).clicked() {
                    if fan::set_auto().is_ok() {
                        notify("Restored automatic firmware fan curve".to_string());
                    }
                }

                ui.add_space(8.0);

                let full_btn = egui::Button::new(RichText::new("Full Turbo (5100 RPM)").strong().color(COLOR_PERF))
                    .fill(SURFACE_CONTAINER)
                    .rounding(ROUNDING_STADIUM)
                    .min_size(Vec2::new(180.0, 38.0));
                if ui.add(full_btn).clicked() {
                    if fan::set_full_speed().is_ok() {
                        notify("Fans set to maximum turbo speed (5100 RPM)".to_string());
                    }
                }

                ui.add_space(8.0);

                let quiet_btn = egui::Button::new(RichText::new("Silent Baseline (1700 RPM)").strong().color(COLOR_QUIET))
                    .fill(SURFACE_CONTAINER)
                    .rounding(ROUNDING_STADIUM)
                    .min_size(Vec2::new(180.0, 38.0));
                if ui.add(quiet_btn).clicked() {
                    let _ = fan::set_fan_target(1, 1700);
                    let _ = fan::set_fan_target(2, 1700);
                    notify("Fans set to silent baseline (1700 RPM)".to_string());
                }
            });

            ui.add_space(22.0);

            // Manual RPM Slider
            if m3_slider_u32(ui, slider_fan_rpm, 1700..=5100, "RPM", "Manual Target Fan Speed", PRIMARY) {
                let _ = fan::set_fan_target(1, *slider_fan_rpm);
                let _ = fan::set_fan_target(2, *slider_fan_rpm);
            }

            ui.add_space(24.0);

            // Dual Blower Live Telemetry Cards
            ui.columns(2, |cols| {
                let fan1_rpm = state.fans.as_ref().and_then(|f| f.fans.first()).map(|f| f.current_rpm).unwrap_or(0);
                let fan2_rpm = state.fans.as_ref().and_then(|f| f.fans.get(1)).map(|f| f.current_rpm).unwrap_or(0);

                // Fan 1 (CPU)
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(22.0))
                    .inner_margin(Margin::same(20.0))
                    .show(&mut cols[0], |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                            draw_icon_fan(ui.painter(), icon_rect, PRIMARY);
                            ui.label(RichText::new("Fan 1 (CPU Cooler)").size(15.0).strong().color(PRIMARY));
                        });
                        ui.add_space(14.0);

                        let ratio = ((fan1_rpm as f32 - 1700.0) / (5100.0 - 1700.0)).clamp(0.0, 1.0);
                        let val_str = format!("{} RPM", fan1_rpm);
                        ui.vertical_centered(|ui| {
                            m3_ring_gauge(ui, ratio, &val_str, "SPEED", PRIMARY, 46.0);
                        });

                        ui.add_space(14.0);
                        m3_progress_bar(ui, ratio, 8.0, PRIMARY);
                        ui.add_space(8.0);
                        ui.label(RichText::new("Operating Bounds: 1700 - 5100 RPM").size(11.0).color(OUTLINE_MUTED));
                    });

                // Fan 2 (GPU)
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(22.0))
                    .inner_margin(Margin::same(20.0))
                    .show(&mut cols[1], |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                            draw_icon_fan(ui.painter(), icon_rect, COLOR_TEAL);
                            ui.label(RichText::new("Fan 2 (GPU Cooler)").size(15.0).strong().color(COLOR_TEAL));
                        });
                        ui.add_space(14.0);

                        let ratio = ((fan2_rpm as f32 - 1700.0) / (5100.0 - 1700.0)).clamp(0.0, 1.0);
                        let val_str = format!("{} RPM", fan2_rpm);
                        ui.vertical_centered(|ui| {
                            m3_ring_gauge(ui, ratio, &val_str, "SPEED", COLOR_TEAL, 46.0);
                        });

                        ui.add_space(14.0);
                        m3_progress_bar(ui, ratio, 8.0, COLOR_TEAL);
                        ui.add_space(8.0);
                        ui.label(RichText::new("Operating Bounds: 1700 - 5100 RPM").size(11.0).color(OUTLINE_MUTED));
                    });
            });
        });
}

