use eframe::egui::{self, Frame, Margin, RichText, Rounding, Vec2};
use legion_cli::hardware::{custom_mode, gpu};

use crate::icons::{draw_icon_gpu, draw_icon_power, draw_icon_thermometer};
use crate::theme::*;
use crate::widgets::{m3_slider_f32, m3_slider_i64};

pub struct PowerSliders {
    pub pl1: i64,
    pub pl2: i64,
    pub pl3: i64,
    pub cpu_temp: i64,
    pub gpu_temp: i64,
    pub ac_offset: i64,
    pub gpu_pl: f32,
}

impl Default for PowerSliders {
    fn default() -> Self {
        Self {
            pl1: 70,
            pl2: 85,
            pl3: 102,
            cpu_temp: 100,
            gpu_temp: 87,
            ac_offset: 55,
            gpu_pl: 80.0,
        }
    }
}

pub fn render_power(
    ui: &mut egui::Ui,
    sliders: &mut PowerSliders,
    notify: &mut impl FnMut(String),
) {
    Frame::none()
        .fill(SURFACE_CONTAINER_LOW)
        .rounding(ROUNDING_SQUIRCLE_XL)
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                draw_icon_power(ui.painter(), icon_rect, PRIMARY);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Custom Mode Hardware Envelopes")
                            .size(20.0)
                            .strong()
                            .color(ON_SURFACE),
                    );
                    ui.label(
                        RichText::new("Material 3 Expressive sliders for CPU sustained/peak power and NVIDIA RTX 5060 TGP tuning")
                            .size(12.0)
                            .color(OUTLINE_MUTED),
                    );
                });
            });

            ui.add_space(20.0);

            ui.columns(2, |cols| {
                // CPU Limits Column
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(22.0))
                    .inner_margin(Margin::same(20.0))
                    .show(&mut cols[0], |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                            draw_icon_thermometer(ui.painter(), icon_rect, PRIMARY);
                            ui.label(RichText::new("AMD Ryzen CPU Package Limits").size(15.0).strong().color(PRIMARY));
                        });
                        ui.add_space(14.0);

                        m3_slider_i64(ui, &mut sliders.pl1, 50..=85, "W", "PL1 Sustained Power Limit", PRIMARY);
                        ui.add_space(12.0);
                        m3_slider_i64(ui, &mut sliders.pl2, 60..=130, "W", "PL2 Slow Package Tracking", PRIMARY);
                        ui.add_space(12.0);
                        m3_slider_i64(ui, &mut sliders.pl3, 70..=150, "W", "PL3 Fast Peak Power", PRIMARY);
                        ui.add_space(12.0);
                        m3_slider_i64(ui, &mut sliders.cpu_temp, 85..=100, "°C", "CPU Thermal Throttle Ceiling", PRIMARY);
                    });

                // GPU Limits Column
                Frame::none()
                    .fill(SURFACE_CONTAINER)
                    .rounding(Rounding::same(22.0))
                    .inner_margin(Margin::same(20.0))
                    .show(&mut cols[1], |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                            draw_icon_gpu(ui.painter(), icon_rect, COLOR_TEAL);
                            ui.label(RichText::new("NVIDIA RTX 5060 Laptop Thermals").size(15.0).strong().color(COLOR_TEAL));
                        });
                        ui.add_space(14.0);

                        m3_slider_f32(ui, &mut sliders.gpu_pl, 5.0..=115.0, "W", "GPU Target TGP Limit", COLOR_TEAL);
                        ui.add_space(12.0);
                        m3_slider_i64(ui, &mut sliders.ac_offset, 10..=85, "W", "GPU AC Baseline Offset", COLOR_TEAL);
                        ui.add_space(12.0);
                        m3_slider_i64(ui, &mut sliders.gpu_temp, 75..=87, "°C", "GPU Thermal Throttle Ceiling", COLOR_TEAL);
                        ui.add_space(16.0);

                        Frame::none()
                            .fill(SURFACE_CONTAINER_HIGH)
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(14.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new("cTGP: 80 W (Fixed) · Dynamic Boost (PPAB): 15 W").size(11.0).color(OUTLINE_MUTED));
                            });
                    });
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                let apply_btn = egui::Button::new(RichText::new("Apply Custom Envelopes").strong().color(ON_PRIMARY))
                    .fill(PRIMARY)
                    .rounding(ROUNDING_STADIUM)
                    .min_size(Vec2::new(190.0, 42.0));

                if ui.add(apply_btn).clicked() {
                    let _ = gpu::set_gpu_tuning_settings(
                        Some(sliders.gpu_pl),
                        Some(sliders.gpu_temp),
                        Some(sliders.ac_offset),
                        None,
                        None,
                        None,
                    );
                    let _ = custom_mode::set_firmware_attribute("ppt_pl1_spl", sliders.pl1);
                    let _ = custom_mode::set_firmware_attribute("ppt_pl2_sppt", sliders.pl2);
                    let _ = custom_mode::set_firmware_attribute("ppt_pl3_fppt", sliders.pl3);
                    let _ = custom_mode::set_firmware_attribute("cpu_temp", sliders.cpu_temp);
                    notify("Custom mode power limits and GPU settings applied to firmware!".to_string());
                }

                ui.add_space(8.0);

                let reset_btn = egui::Button::new(RichText::new("Reset to Balanced Defaults").color(ON_SURFACE))
                    .fill(SURFACE_CONTAINER_HIGH)
                    .rounding(ROUNDING_STADIUM)
                    .min_size(Vec2::new(180.0, 42.0));

                if ui.add(reset_btn).clicked() {
                    *sliders = PowerSliders::default();
                    notify("Reset sliders to factory defaults".to_string());
                }
            });
        });
}

