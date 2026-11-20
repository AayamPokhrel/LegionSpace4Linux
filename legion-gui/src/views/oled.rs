use eframe::egui::{self, Frame, Margin, RichText, Rounding, Vec2};
use legion_cli::hardware::oled;

use crate::icons::draw_icon_display;
use crate::telemetry::AppStateSnapshot;
use crate::theme::*;
use crate::widgets::{m3_progress_bar, m3_switch};

pub fn render_oled(
    ui: &mut egui::Ui,
    state: &AppStateSnapshot,
    notify: &mut impl FnMut(String),
) {
    let oled_cfg = &state.oled;

    // ==============================================================================
    // 1. Hero Card: Display & OLED Care Suite
    // ==============================================================================
    Frame::none()
        .fill(SURFACE_CONTAINER_LOW)
        .rounding(ROUNDING_SQUIRCLE_XL)
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                draw_icon_display(ui.painter(), icon_rect, PRIMARY);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Display & OLED Care Protection Suite")
                            .size(20.0)
                            .strong()
                            .color(ON_SURFACE),
                    );
                    ui.label(
                        RichText::new("Official Lenovo anti-burn algorithms, pixel shift micro-translations, and LCD overdrive")
                            .size(12.0)
                            .color(OUTLINE_MUTED),
                    );
                });
            });

            ui.add_space(20.0);

            // Two main control columns
            ui.columns(2, |cols| {
                // Column 1: OLED Anti-Burn Features
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(22.0))
                    .inner_margin(Margin::same(20.0))
                    .show(&mut cols[0], |ui| {
                        ui.label(RichText::new("OLED Burn-in Prevention").size(15.0).strong().color(PRIMARY));
                        ui.add_space(14.0);

                        // Master Toggle
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Master OLED Protection").strong().color(ON_SURFACE));
                                ui.label(RichText::new("Enables global anti-burn mitigation engine").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut master = oled_cfg.master_oled_protection;
                                if m3_switch(ui, &mut master) {
                                    if oled::update_oled_settings(Some(master), None, None, None, None, None).is_ok() {
                                        notify(format!("Master OLED Protection: {}", if master { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                        });

                        ui.add_space(16.0);

                        // Pixel Shift Switch
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Pixel Shift (Auto Shift)").strong().color(ON_SURFACE));
                                ui.label(RichText::new("Micro-shifts pixels to distribute sub-pixel wear").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut shift = oled_cfg.pixel_shift_enabled;
                                if m3_switch(ui, &mut shift) {
                                    if oled::update_oled_settings(None, Some(shift), None, None, None, None).is_ok() {
                                        notify(format!("Pixel Shift: {}", if shift { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                        });

                        ui.add_space(16.0);

                        // Static Taskbar Dimming Switch
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Static Taskbar Dimming").strong().color(ON_SURFACE));
                                ui.label(RichText::new("Dims persistent desktop bars and fixed UI icons").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut dim = oled_cfg.taskbar_dimming_enabled;
                                if m3_switch(ui, &mut dim) {
                                    if oled::update_oled_settings(None, None, Some(dim), None, None, None).is_ok() {
                                        notify(format!("Taskbar Dimming: {}", if dim { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                        });

                        ui.add_space(16.0);

                        // Default Dark Mode Enforcer
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Default Dark Mode (GNOME)").strong().color(ON_SURFACE));
                                ui.label(RichText::new("Syncs system theme to turn off pure black OLED pixels").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut dark = oled_cfg.default_dark_mode;
                                if m3_switch(ui, &mut dark) {
                                    if oled::update_oled_settings(None, None, None, Some(dark), None, None).is_ok() {
                                        notify(format!("Default Dark Mode: {}", if dark { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                        });
                    });

                // Column 2: Display Dynamics & LCD OverDrive
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(22.0))
                    .inner_margin(Margin::same(20.0))
                    .show(&mut cols[1], |ui| {
                        ui.label(RichText::new("Panel Response & Dynamics").size(15.0).strong().color(COLOR_TEAL));
                        ui.add_space(14.0);

                        // LCD OverDrive Switch
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("LCD Panel OverDrive (OD)").strong().color(ON_SURFACE));
                                ui.label(RichText::new("Accelerates gray-to-gray response time for fast gaming").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut od = oled_cfg.lcd_overdrive_enabled;
                                if m3_switch(ui, &mut od) {
                                    if oled::update_oled_settings(None, None, None, None, Some(od), None).is_ok() {
                                        notify(format!("LCD OverDrive: {}", if od { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                        });

                        ui.add_space(16.0);

                        // Adaptive Refresh Rate (Fn+R)
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Adaptive Refresh Rate (Fn+R)").strong().color(ON_SURFACE));
                                ui.label(RichText::new("Dynamic 60 Hz battery saver <-> 165/240 Hz AC").size(11.0).color(OUTLINE_MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut arr = oled_cfg.adaptive_refresh_rate;
                                if m3_switch(ui, &mut arr) {
                                    if oled::update_oled_settings(None, None, None, None, None, Some(arr)).is_ok() {
                                        notify(format!("Adaptive Refresh Rate: {}", if arr { "Enabled" } else { "Disabled" }));
                                    }
                                }
                            });
                        });

                        ui.add_space(18.0);

                        Frame::none()
                            .fill(SURFACE_CONTAINER_HIGH)
                            .rounding(Rounding::same(14.0))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Lock Screen Screen-Off Timeout: 30 minutes (1800s)").size(11.0).color(OUTLINE_MUTED));
                                ui.label(RichText::new("Auto-dim active when system is idle").size(11.0).color(OUTLINE_MUTED));
                            });
                    });
            });

            ui.add_space(20.0);

            // ==============================================================================
            // 2. 4-Hour Panel Pixel Clean Cycle Maintenance Card
            // ==============================================================================
            Frame::none()
                .fill(SURFACE_CONTAINER)
                .rounding(Rounding::same(22.0))
                .inner_margin(Margin::same(20.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Panel Pixel Cleaning & Calibration Cycle").size(16.0).strong().color(ON_SURFACE));
                            ui.label(RichText::new("Lenovo recommendation: Run a clean cycle every 4 hours of active display on-time to calibrate pixel brightness uniformity").size(12.0).color(OUTLINE_MUTED));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let clean_due = oled_cfg.clean_reminder_due;
                            let badge_col = if clean_due { COLOR_PERF } else { COLOR_SUCCESS };
                            let badge_text = if clean_due { "CYCLE DUE" } else { "OPTIMAL" };
                            Frame::none()
                                .fill(SURFACE_CONTAINER_HIGH)
                                .rounding(ROUNDING_STADIUM)
                                .inner_margin(Margin::symmetric(14.0, 6.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(badge_text).size(11.0).strong().color(badge_col));
                                });
                        });
                    });

                    ui.add_space(14.0);

                    // Progress bar for 4-hour cycle (14,400s)
                    let hours_elapsed = oled_cfg.screen_on_seconds as f32 / 3600.0;
                    let progress = (oled_cfg.screen_on_seconds as f32 / 14400.0).clamp(0.0, 1.0);
                    let bar_color = if oled_cfg.clean_reminder_due { COLOR_PERF } else { PRIMARY };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Active On-Time: {:.1} hrs / 4.0 hrs", hours_elapsed)).size(13.0).strong().color(ON_SURFACE));
                    });
                    ui.add_space(6.0);
                    m3_progress_bar(ui, progress, 10.0, bar_color);

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let clean_btn = egui::Button::new(RichText::new("Perform Pixel Clean & Reset Timer").strong().color(ON_PRIMARY))
                            .fill(PRIMARY)
                            .rounding(ROUNDING_STADIUM)
                            .min_size(Vec2::new(240.0, 40.0));

                        if ui.add(clean_btn).clicked() {
                            if oled::reset_pixel_clean_timer().is_ok() {
                                notify("OLED Pixel Clean cycle completed and 4-hour timer reset!".to_string());
                            }
                        }
                    });
                });
        });
}

