use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Vec2};
use legion_cli::hardware::lighting::{self, LightingEffect, RgbColor};

use crate::icons::draw_icon_keyboard;
use crate::theme::*;
use crate::widgets::m3_slider_u8;

pub struct LightingState {
    pub zone_target: u8,
    pub rgb_color: [u8; 3],
    pub brightness: u8,
    #[allow(dead_code)]
    pub speed: u8,
}

impl Default for LightingState {
    fn default() -> Self {
        Self {
            zone_target: 0,
            rgb_color: [0, 211, 254], // Legion Cyan
            brightness: 3,
            speed: 2,
        }
    }
}

pub fn render_lighting(
    ui: &mut egui::Ui,
    state: &mut LightingState,
    notify: &mut impl FnMut(String),
) {
    Frame::none()
        .fill(SURFACE_CONTAINER_LOW)
        .rounding(ROUNDING_SQUIRCLE_XL)
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                draw_icon_keyboard(ui.painter(), icon_rect, PRIMARY);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("4-Zone RGB Keyboard Lighting").size(20.0).strong().color(ON_SURFACE));
                    ui.label(RichText::new("Direct ITE 8258 USB HID (048d:c195) 0xCC report protocol controller").size(12.0).color(OUTLINE_MUTED));
                });
            });

            ui.add_space(20.0);

            // Quick Lighting Presets
            ui.horizontal(|ui| {
                let presets = [
                    ("legion-blue", "Legion Blue", Color32::from_rgb(0, 211, 254)),
                    ("cyberpunk", "Cyberpunk", Color32::from_rgb(255, 0, 127)),
                    ("crimson", "Crimson", Color32::from_rgb(255, 42, 42)),
                    ("spectrum", "Spectrum Wave", PRIMARY),
                    ("stealth", "Stealth (Off)", Color32::GRAY),
                ];

                for (id, name, col) in presets {
                    let btn = egui::Button::new(RichText::new(name).size(12.0).strong().color(col))
                        .fill(SURFACE_CONTAINER)
                        .rounding(ROUNDING_STADIUM)
                        .min_size(Vec2::new(130.0, 38.0));
                    if ui.add(btn).clicked() {
                        if lighting::apply_preset(id).is_ok() {
                            notify(format!("Applied RGB preset: {}", name));
                        }
                    }
                }
            });

            ui.add_space(22.0);

            // Visual Keyboard 4-Zone Representation
            ui.label(RichText::new("Zone Layout").size(14.0).strong().color(ON_SURFACE));
            ui.add_space(8.0);
            ui.columns(4, |cols| {
                let zone_labels = [
                    ("Zone 1", "WASD / Left"),
                    ("Zone 2", "Center-Left"),
                    ("Zone 3", "Center-Right"),
                    ("Zone 4", "Numpad / Right"),
                ];
                for (i, (zn, desc)) in zone_labels.iter().enumerate() {
                    let is_sel = state.zone_target == (i as u8 + 1) || state.zone_target == 0;
                    Frame::none()
                        .fill(if is_sel { SURFACE_CONTAINER_HIGH } else { SURFACE_CONTAINER })
                        .stroke(if is_sel { Stroke::new(1.5_f32, PRIMARY) } else { Stroke::NONE })
                        .rounding(Rounding::same(16.0))
                        .inner_margin(Margin::same(14.0))
                        .show(&mut cols[i], |ui| {
                            ui.label(RichText::new(*zn).size(13.0).strong().color(ON_SURFACE));
                            ui.label(RichText::new(*desc).size(11.0).color(OUTLINE_MUTED));
                        });
                }
            });

            ui.add_space(20.0);

            // Target Zone Selector Pills
            ui.horizontal(|ui| {
                ui.label(RichText::new("Target Zone:").strong().color(ON_SURFACE));
                let zones = [(0, "All Zones"), (1, "Zone 1"), (2, "Zone 2"), (3, "Zone 3"), (4, "Zone 4")];
                for (z, label) in zones {
                    let is_active = state.zone_target == z;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .strong()
                            .color(if is_active { ON_PRIMARY } else { ON_SURFACE }),
                    )
                    .fill(if is_active { PRIMARY } else { SURFACE_CONTAINER })
                    .rounding(ROUNDING_STADIUM);
                    if ui.add(btn).clicked() {
                        state.zone_target = z;
                    }
                }
            });

            ui.add_space(18.0);

            // Color Picker & Brightness Slider
            ui.horizontal(|ui| {
                ui.label(RichText::new("Color:").strong().color(ON_SURFACE));
                ui.color_edit_button_srgb(&mut state.rgb_color);

                ui.add_space(14.0);

                let apply_color_btn = egui::Button::new(
                    RichText::new("Apply Solid Color").strong().color(ON_PRIMARY),
                )
                .fill(PRIMARY)
                .rounding(ROUNDING_STADIUM)
                .min_size(Vec2::new(150.0, 36.0));

                if ui.add(apply_color_btn).clicked() {
                    let color = RgbColor::new(state.rgb_color[0], state.rgb_color[1], state.rgb_color[2]);
                    if state.zone_target == 0 {
                        let _ = lighting::set_all_zones(LightingEffect::Static, 2, state.brightness, color);
                    } else {
                        let _ = lighting::set_zone_color(state.zone_target, LightingEffect::Static, 2, state.brightness, color);
                        let _ = lighting::commit_rgb_profile(1, state.brightness);
                    }
                    notify("Custom RGB color applied to keyboard".to_string());
                }
            });

            ui.add_space(18.0);
            m3_slider_u8(ui, &mut state.brightness, 0..=3, "/ 3", "Keyboard Backlight Brightness", PRIMARY);
        });
}
