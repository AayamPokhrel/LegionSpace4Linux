use eframe::egui::{self, epaint::PathShape, Color32, Frame, Margin, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use crate::icons::draw_icon_check;
use crate::theme::*;

// ==============================================================================
// Custom Material 3 Expressive Widgets
// ==============================================================================

/// Material 3 Expressive Stadium Switch (52x32dp with sliding thumb & checkmark)
pub fn m3_switch(ui: &mut egui::Ui, value: &mut bool) -> bool {
    let desired_size = Vec2::new(52.0, 32.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    let painter = ui.painter();
    let is_on = *value;
    let track_rounding = Rounding::same(16.0);

    let track_color = if is_on {
        PRIMARY
    } else {
        SURFACE_CONTAINER_HIGHEST
    };
    painter.rect_filled(rect, track_rounding, track_color);

    if !is_on {
        painter.rect_stroke(rect, track_rounding, Stroke::new(1.5_f32, OUTLINE_MUTED));
    }

    let thumb_radius = if is_on { 11.5 } else { 8.0 };
    let thumb_center_x = if is_on {
        rect.max.x - 16.0
    } else {
        rect.min.x + 16.0
    };
    let thumb_center = Pos2::new(thumb_center_x, rect.center().y);
    let thumb_color = if is_on { ON_PRIMARY } else { ON_SURFACE_VARIANT };

    painter.circle_filled(thumb_center, thumb_radius, thumb_color);

    if is_on {
        draw_icon_check(painter, thumb_center, 12.0, PRIMARY);
    }

    response.changed()
}

/// Material 3 Expressive Continuous Thick Slider
pub fn m3_slider_f64(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
    label: &str,
    accent: Color32,
) -> bool {
    let min_f = *range.start();
    let max_f = *range.end();
    let cur_f = *value;
    let ratio = ((cur_f - min_f) / (max_f - min_f)).clamp(0.0, 1.0) as f32;

    let mut changed = false;

    ui.vertical(|ui| {
        // Label & Live Value Stadium Badge
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(13.0).strong().color(ON_SURFACE));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let badge_text = format!("{:.0} {}", cur_f, suffix);
                Frame::none()
                    .fill(SURFACE_CONTAINER_HIGHEST)
                    .rounding(ROUNDING_STADIUM)
                    .inner_margin(Margin::symmetric(10.0, 3.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(badge_text).size(12.0).strong().color(accent));
                    });
            });
        });

        ui.add_space(6.0);

        // 16dp Thick Stadium Pill Track
        let track_height = 16.0;
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), track_height),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter();

        if response.clicked() || response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                let new_ratio = ((mouse_pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                *value = min_f + (new_ratio as f64) * (max_f - min_f);
                changed = true;
            }
        }

        // Inactive background track
        painter.rect_filled(rect, Rounding::same(track_height / 2.0), SURFACE_CONTAINER_HIGHEST);

        // Active filled progress track
        let active_width = rect.width() * ratio;
        if active_width > 4.0 {
            let active_rect = Rect::from_min_size(rect.min, Vec2::new(active_width, track_height));
            painter.rect_filled(active_rect, Rounding::same(track_height / 2.0), accent);
        }

        // Handle thumb
        let thumb_x = rect.min.x + active_width;
        let thumb_rect = Rect::from_center_size(
            Pos2::new(thumb_x, rect.center().y),
            Vec2::new(6.0, track_height + 6.0),
        );
        painter.rect_filled(thumb_rect, Rounding::same(3.0), Color32::WHITE);
    });

    changed
}

pub fn m3_slider_i64(
    ui: &mut egui::Ui,
    value: &mut i64,
    range: std::ops::RangeInclusive<i64>,
    suffix: &str,
    label: &str,
    accent: Color32,
) -> bool {
    let mut f = *value as f64;
    let changed = m3_slider_f64(ui, &mut f, (*range.start() as f64)..=(*range.end() as f64), suffix, label, accent);
    if changed {
        *value = f.round() as i64;
    }
    changed
}

pub fn m3_slider_f32(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    label: &str,
    accent: Color32,
) -> bool {
    let mut f = *value as f64;
    let changed = m3_slider_f64(ui, &mut f, (*range.start() as f64)..=(*range.end() as f64), suffix, label, accent);
    if changed {
        *value = f as f32;
    }
    changed
}

pub fn m3_slider_u32(
    ui: &mut egui::Ui,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    suffix: &str,
    label: &str,
    accent: Color32,
) -> bool {
    let mut f = *value as f64;
    let changed = m3_slider_f64(ui, &mut f, (*range.start() as f64)..=(*range.end() as f64), suffix, label, accent);
    if changed {
        *value = f.round() as u32;
    }
    changed
}

pub fn m3_slider_u8(
    ui: &mut egui::Ui,
    value: &mut u8,
    range: std::ops::RangeInclusive<u8>,
    suffix: &str,
    label: &str,
    accent: Color32,
) -> bool {
    let mut f = *value as f64;
    let changed = m3_slider_f64(ui, &mut f, (*range.start() as f64)..=(*range.end() as f64), suffix, label, accent);
    if changed {
        *value = f.round() as u8;
    }
    changed
}

/// Material 3 Progress Bar with rounded stadium caps
pub fn m3_progress_bar(ui: &mut egui::Ui, ratio: f32, height: f32, fill_color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter();

    // Inactive track
    painter.rect_filled(rect, Rounding::same(height / 2.0), SURFACE_CONTAINER_HIGHEST);

    // Active track
    let active_w = (rect.width() * ratio.clamp(0.0, 1.0)).max(height);
    let active_rect = Rect::from_min_size(rect.min, Vec2::new(active_w, height));
    painter.rect_filled(active_rect, Rounding::same(height / 2.0), fill_color);
}

/// Material 3 Expressive Circular Arc Meter / Ring Gauge (Speedometer)
pub fn m3_ring_gauge(
    ui: &mut egui::Ui,
    ratio: f32,
    center_text: &str,
    sub_text: &str,
    accent: Color32,
    radius: f32,
) {
    let diameter = radius * 2.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter + 12.0), egui::Sense::hover());
    let center = rect.center();
    let painter = ui.painter();

    let start_angle = 140.0f32.to_radians();
    let total_sweep = 260.0f32.to_radians();
    let stroke_w = 8.5_f32;

    // Background Inactive Arc (260 degrees)
    let steps = 40;
    let bg_pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let a = start_angle + (i as f32 / steps as f32) * total_sweep;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect();
    painter.add(PathShape::line(bg_pts, Stroke::new(stroke_w, SURFACE_CONTAINER_HIGHEST)));

    // Active Progress Arc
    let clamped_ratio = ratio.clamp(0.0, 1.0);
    if clamped_ratio > 0.02 {
        let active_steps = (steps as f32 * clamped_ratio).ceil() as usize;
        let active_sweep = total_sweep * clamped_ratio;
        let fg_pts: Vec<Pos2> = (0..=active_steps)
            .map(|i| {
                let a = start_angle + (i as f32 / active_steps as f32) * active_sweep;
                center + Vec2::new(a.cos(), a.sin()) * radius
            })
            .collect();
        painter.add(PathShape::line(fg_pts, Stroke::new(stroke_w, accent)));
    }

    // Center Display Numbers
    painter.text(
        center - Vec2::new(0.0, 6.0),
        egui::Align2::CENTER_CENTER,
        center_text,
        egui::FontId::proportional(22.0),
        ON_SURFACE,
    );
    // Subtitle
    painter.text(
        center + Vec2::new(0.0, 13.0),
        egui::Align2::CENTER_CENTER,
        sub_text,
        egui::FontId::proportional(11.0),
        OUTLINE_MUTED,
    );
}
