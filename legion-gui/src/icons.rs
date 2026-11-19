use eframe::egui::{self, epaint::PathShape, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

// ==============================================================================
// Native Vector Icon Painters (Zero Missing Glyphs / Tofu Proof)
// Scalable, DPI-independent, authentic Google Material 3 Expressive geometry
// ==============================================================================

/// 2x2 Squircle Tiles (Material 3 Dashboard Icon)
pub fn draw_icon_dashboard(painter: &egui::Painter, rect: Rect, color: Color32) {
    let pad = rect.width() * 0.16;
    let size = (rect.width() - pad * 3.0) / 2.0;
    let r = Rounding::same(size * 0.32);

    let p0 = rect.min + Vec2::new(pad, pad);
    let p1 = p0 + Vec2::new(size + pad, 0.0);
    let p2 = p0 + Vec2::new(0.0, size + pad);
    let p3 = p0 + Vec2::new(size + pad, size + pad);

    for p in [p0, p1, p2, p3] {
        painter.rect_filled(Rect::from_min_size(p, Vec2::splat(size)), r, color);
    }
}

/// Dynamic Lightning Bolt (Power / GPU Tuning Icon)
pub fn draw_icon_power(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.42;
    let h = rect.height() * 0.48;

    let points = vec![
        Pos2::new(c.x + w * 0.1, c.y - h),
        Pos2::new(c.x - w * 0.7, c.y + h * 0.05),
        Pos2::new(c.x - w * 0.05, c.y + h * 0.05),
        Pos2::new(c.x - w * 0.25, c.y + h),
        Pos2::new(c.x + w * 0.75, c.y - h * 0.1),
        Pos2::new(c.x + w * 0.1, c.y - h * 0.1),
    ];

    painter.add(PathShape::convex_polygon(points, color, Stroke::NONE));
}

/// 3-Blade Aerodynamic Turbine (Cooling Fans Icon)
pub fn draw_icon_fan(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width() * 0.44;

    // Center hub
    painter.circle_filled(c, r * 0.26, color);

    // 3 Aerodynamic curved blades
    for angle_deg in [0.0f32, 120.0f32, 240.0f32] {
        let a1 = angle_deg.to_radians();
        let a2 = (angle_deg + 45.0).to_radians();

        let p_start = c + Vec2::new(a1.cos(), a1.sin()) * (r * 0.26);
        let p_tip = c + Vec2::new(a1.cos(), a1.sin()) * r;
        let p_curve = c + Vec2::new(a2.cos(), a2.sin()) * (r * 0.75);

        let blade_pts = vec![c, p_start, p_tip, p_curve];
        painter.add(PathShape::convex_polygon(blade_pts, color, Stroke::NONE));
    }
}

/// Battery Capsule with Live Charge Level (Battery Care Icon)
pub fn draw_icon_battery(painter: &egui::Painter, rect: Rect, color: Color32, pct: f32) {
    let c = rect.center();
    let w = rect.width() * 0.72;
    let h = rect.height() * 0.44;

    let body_rect = Rect::from_center_size(c - Vec2::new(w * 0.06, 0.0), Vec2::new(w, h));
    let r = Rounding::same(h * 0.25);

    // Battery Shell Outline
    painter.rect_stroke(body_rect, r, Stroke::new(1.8_f32, color));

    // Positive Terminal Nub
    let cap_w = w * 0.08;
    let cap_h = h * 0.42;
    let cap_pos = Pos2::new(body_rect.max.x + 1.0, c.y - cap_h / 2.0);
    painter.rect_filled(
        Rect::from_min_size(cap_pos, Vec2::new(cap_w, cap_h)),
        Rounding::same(2.0),
        color,
    );

    // Inner Fill
    let inner_pad = 2.5;
    let max_fill_w = (body_rect.width() - inner_pad * 2.0).max(0.0);
    let fill_w = (max_fill_w * (pct / 100.0).clamp(0.0, 1.0)).max(2.0);
    let fill_rect = Rect::from_min_size(
        body_rect.min + Vec2::new(inner_pad, inner_pad),
        Vec2::new(fill_w, body_rect.height() - inner_pad * 2.0),
    );
    painter.rect_filled(fill_rect, Rounding::same(2.0), color);
}

/// Keyboard with 4-Zone Matrix (RGB Lighting Icon)
pub fn draw_icon_keyboard(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.82;
    let h = rect.height() * 0.50;

    let body = Rect::from_center_size(c, Vec2::new(w, h));
    painter.rect_filled(body, Rounding::same(4.0), color);

    // Inner keys cutout / contrast dots
    let key_pad = 3.0;
    let inner = body.shrink(key_pad);
    let seg_w = (inner.width() - 3.0 * 2.0) / 4.0;
    let seg_h = inner.height();

    for i in 0..4 {
        let x = inner.min.x + (i as f32) * (seg_w + 2.0);
        let key_rect = Rect::from_min_size(Pos2::new(x, inner.min.y), Vec2::new(seg_w, seg_h));
        painter.rect_filled(key_rect, Rounding::same(2.0), Color32::from_black_alpha(160));
    }
}

/// Thermometer Stem & Bulb
pub fn draw_icon_thermometer(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let stem_w = rect.width() * 0.22;
    let bulb_r = rect.width() * 0.26;

    let stem_rect = Rect::from_min_max(
        Pos2::new(c.x - stem_w / 2.0, rect.min.y + rect.height() * 0.12),
        Pos2::new(c.x + stem_w / 2.0, c.y + bulb_r * 0.5),
    );
    painter.rect_filled(stem_rect, Rounding::same(stem_w / 2.0), color);
    painter.circle_filled(Pos2::new(c.x, c.y + bulb_r * 0.8), bulb_r, color);
}

/// RAM Memory Module
pub fn draw_icon_ram(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.84;
    let h = rect.height() * 0.44;

    let body = Rect::from_center_size(c, Vec2::new(w, h));
    painter.rect_filled(body, Rounding::same(3.0), color);

    // IC chips
    let chip_w = w * 0.16;
    let chip_h = h * 0.55;
    for i in 0..3 {
        let cx = body.min.x + w * 0.22 + (i as f32) * (chip_w * 1.55);
        let chip = Rect::from_center_size(Pos2::new(cx, c.y), Vec2::new(chip_w, chip_h));
        painter.rect_filled(chip, Rounding::same(1.5), Color32::from_black_alpha(150));
    }
}

/// GPU Silicon Die
pub fn draw_icon_gpu(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = rect.width() * 0.70;

    let outer = Rect::from_center_size(c, Vec2::splat(s));
    painter.rect_filled(outer, Rounding::same(4.0), color);

    let inner = Rect::from_center_size(c, Vec2::splat(s * 0.55));
    painter.rect_filled(inner, Rounding::same(2.5), Color32::from_black_alpha(170));
}

/// Camera Privacy & E-Shutter
pub fn draw_icon_camera(painter: &egui::Painter, rect: Rect, color: Color32, active: bool) {
    let c = rect.center();
    let w = rect.width() * 0.76;
    let h = rect.height() * 0.52;

    let body = Rect::from_center_size(c, Vec2::new(w, h));
    painter.rect_filled(body, Rounding::same(4.0), color);

    let lens_c = Pos2::new(c.x, c.y + 1.0);
    painter.circle_filled(lens_c, h * 0.35, Color32::from_black_alpha(170));
    painter.circle_filled(lens_c, h * 0.18, color);

    if !active {
        // Privacy diagonal slash line
        let p1 = body.left_top() - Vec2::new(2.0, 2.0);
        let p2 = body.right_bottom() + Vec2::new(2.0, 2.0);
        painter.line_segment([p1, p2], Stroke::new(2.5_f32, Color32::from_rgb(255, 110, 110)));
    }
}

/// Eco Leaf (Quiet Mode)
pub fn draw_icon_leaf(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width() * 0.42;

    let p_bottom = c + Vec2::new(-r * 0.7, r * 0.7);
    let p_top = c + Vec2::new(r * 0.7, -r * 0.7);
    let p_side1 = c + Vec2::new(r * 0.1, r * 0.8);
    let p_side2 = c + Vec2::new(-r * 0.8, -r * 0.1);

    let leaf_pts = vec![p_bottom, p_side1, p_top, p_side2];
    painter.add(PathShape::convex_polygon(leaf_pts, color, Stroke::NONE));
    painter.line_segment([p_bottom, p_top], Stroke::new(1.8_f32, Color32::from_black_alpha(100)));
}

/// Speedometer Gauge (Balanced Mode)
pub fn draw_icon_gauge(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width() * 0.42;

    painter.circle_stroke(c, r, Stroke::new(2.0_f32, color));
    let needle_end = c + Vec2::new(0.6 * r, -0.6 * r);
    painter.line_segment([c, needle_end], Stroke::new(2.2_f32, color));
    painter.circle_filled(c, 2.8, color);
}

/// Performance Flame (Turbo Mode)
pub fn draw_icon_flame(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.38;
    let h = rect.height() * 0.48;

    let points = vec![
        Pos2::new(c.x, c.y - h),
        Pos2::new(c.x + w * 0.6, c.y - h * 0.2),
        Pos2::new(c.x + w * 0.8, c.y + h * 0.5),
        Pos2::new(c.x + w * 0.3, c.y + h),
        Pos2::new(c.x - w * 0.3, c.y + h),
        Pos2::new(c.x - w * 0.8, c.y + h * 0.4),
        Pos2::new(c.x - w * 0.4, c.y - h * 0.1),
    ];

    painter.add(PathShape::convex_polygon(points, color, Stroke::NONE));
}

/// Equalizer Sliders (Custom Mode)
pub fn draw_icon_sliders(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let w = rect.width() * 0.72;
    let h = rect.height() * 0.50;

    let y1 = c.y - h * 0.35;
    let y2 = c.y + h * 0.35;

    // Line 1
    painter.line_segment([Pos2::new(c.x - w / 2.0, y1), Pos2::new(c.x + w / 2.0, y1)], Stroke::new(1.8_f32, color));
    painter.circle_filled(Pos2::new(c.x - w * 0.18, y1), 3.2, color);

    // Line 2
    painter.line_segment([Pos2::new(c.x - w / 2.0, y2), Pos2::new(c.x + w / 2.0, y2)], Stroke::new(1.8_f32, color));
    painter.circle_filled(Pos2::new(c.x + w * 0.18, y2), 3.2, color);
}

/// Bold Material 3 Checkmark
pub fn draw_icon_check(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let p1 = center + Vec2::new(-size * 0.35, 0.0);
    let p2 = center + Vec2::new(-size * 0.08, size * 0.32);
    let p3 = center + Vec2::new(size * 0.40, -size * 0.32);

    painter.line_segment([p1, p2], Stroke::new(2.4_f32, color));
    painter.line_segment([p2, p3], Stroke::new(2.4_f32, color));
}

/// Authentic Material 3 Expressive 4-Lobe Clover / Scalloped Squircle
pub fn draw_scalloped_clover(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let lobe_r = radius * 0.65;
    let offset = radius * 0.42;

    // 4 Overlapping circular lobes (Google Keep / M3E Widget Shape)
    painter.circle_filled(center + Vec2::new(-offset, 0.0), lobe_r, color);
    painter.circle_filled(center + Vec2::new(offset, 0.0), lobe_r, color);
    painter.circle_filled(center + Vec2::new(0.0, -offset), lobe_r, color);
    painter.circle_filled(center + Vec2::new(0.0, offset), lobe_r, color);

    // Central core squircle fill
    let core_rect = Rect::from_center_size(center, Vec2::splat(radius * 1.05));
    painter.rect_filled(core_rect, Rounding::same(radius * 0.35), color);
}

/// Authentic Material 3 Expressive 8-Lobe Scalloped Flower Badge (Android 15 Avatar)
pub fn draw_scalloped_flower(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let lobe_r = radius * 0.46;
    let dist = radius * 0.60;

    for i in 0..8 {
        let angle = (i as f32 * 45.0).to_radians();
        let p = center + Vec2::new(angle.cos(), angle.sin()) * dist;
        painter.circle_filled(p, lobe_r, color);
    }
    painter.circle_filled(center, radius * 0.72, color);
}

/// Lenovo Legion 3-Spoke Star Emblem
pub fn draw_legion_star(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    for angle_deg in [-90.0f32, 30.0f32, 150.0f32] {
        let rad = angle_deg.to_radians();
        let end = center + Vec2::new(rad.cos(), rad.sin()) * (radius * 0.88);
        painter.line_segment([center, end], Stroke::new(2.8_f32, color));
    }
    painter.circle_filled(center, radius * 0.24, Color32::from_black_alpha(200));
    painter.circle_stroke(center, radius * 0.24, Stroke::new(1.4_f32, color));
}
