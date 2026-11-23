mod icons;
mod telemetry;
mod theme;
mod views;
mod widgets;

use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use icons::*;
use telemetry::*;
use theme::*;
use views::lighting::LightingState;
use views::power::PowerSliders;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum AppTab {
    Dashboard,
    CustomMode,
    Oled,
    Cooling,
    Battery,
    Lighting,
}

pub struct LegionSpaceApp {
    state: AppStateSnapshot,
    rx: Receiver<AppStateSnapshot>,
    current_tab: AppTab,
    status_msg: Option<(String, Instant)>,

    // Subview persistent states
    power_sliders: PowerSliders,
    slider_fan_rpm: u32,
    lighting_state: LightingState,
}

impl LegionSpaceApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_system_fonts(&cc.egui_ctx);
        let (rx, initial) = start_telemetry_thread();

        Self {
            state: initial,
            rx,
            current_tab: AppTab::Dashboard,
            status_msg: None,
            power_sliders: PowerSliders::default(),
            slider_fan_rpm: 3500,
            lighting_state: LightingState::default(),
        }
    }

    fn notify(&mut self, text: impl Into<String>) {
        self.status_msg = Some((text.into(), Instant::now()));
    }
}

impl eframe::App for LegionSpaceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(new_state) = self.rx.try_recv() {
            self.state = new_state;
        }

        // ==============================================================================
        // Global Material 3 Dark Scheme Visuals
        // Borderless, purely tonal elevation, authentic squircle rounding
        // ==============================================================================
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(ON_SURFACE);
        visuals.panel_fill = SURFACE_DIM;
        visuals.window_fill = SURFACE;
        visuals.faint_bg_color = SURFACE_CONTAINER_LOW;
        visuals.extreme_bg_color = SURFACE_DIM;

        // Zero harsh borders across all egui default widgets
        visuals.widgets.noninteractive.bg_fill = SURFACE_CONTAINER;
        visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, ON_SURFACE);
        visuals.widgets.noninteractive.rounding = Rounding::same(20.0);

        visuals.widgets.inactive.bg_fill = SURFACE_CONTAINER;
        visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, ON_SURFACE);
        visuals.widgets.inactive.rounding = Rounding::same(20.0);

        visuals.widgets.hovered.bg_fill = SURFACE_CONTAINER_HIGH;
        visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.hovered.rounding = Rounding::same(20.0);

        visuals.widgets.active.bg_fill = PRIMARY;
        visuals.widgets.active.bg_stroke = Stroke::NONE;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, ON_PRIMARY);
        visuals.widgets.active.rounding = Rounding::same(20.0);

        ctx.set_visuals(visuals);

        // ==============================================================================
        // Material 3 Navigation Rail (Vertical Left Drawer)
        // Authentic to Google M3 Tablet/Desktop Architecture & Legion Space
        // ==============================================================================
        egui::SidePanel::left("m3_nav_rail")
            .exact_width(235.0)
            .frame(
                Frame::none()
                    .fill(SURFACE_CONTAINER_LOW)
                    .inner_margin(Margin::symmetric(16.0, 20.0)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Legion Branding Header with Scalloped M3 Clover
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::hover());
                        let center = icon_rect.center();
                        draw_scalloped_clover(ui.painter(), center, 18.0, PRIMARY);
                        draw_legion_star(ui.painter(), center, 14.0, ON_PRIMARY);

                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Legion Space")
                                    .size(17.0)
                                    .strong()
                                    .color(ON_SURFACE),
                            );
                            ui.label(
                                RichText::new("Material 3 Expressive")
                                    .size(11.0)
                                    .color(PRIMARY),
                            );
                        });
                    });

                    ui.add_space(26.0);

                    // Navigation Rail Items (Full Stadium Pills)
                    let nav_items = [
                        (AppTab::Dashboard, "Dashboard", 1),
                        (AppTab::CustomMode, "Power & GPU", 2),
                        (AppTab::Oled, "Display & OLED", 6),
                        (AppTab::Cooling, "Cooling Fans", 3),
                        (AppTab::Battery, "Battery & Care", 4),
                        (AppTab::Lighting, "Keyboard RGB", 5),
                    ];

                    for (tab, title, icon_kind) in nav_items {
                        let is_active = self.current_tab == tab;
                        let item_rect = ui.allocate_exact_size(Vec2::new(203.0, 48.0), egui::Sense::click());
                        let painter = ui.painter();

                        if item_rect.1.clicked() {
                            self.current_tab = tab;
                        }

                        let bg_color = if is_active {
                            PRIMARY_CONTAINER
                        } else if item_rect.1.hovered() {
                            SURFACE_CONTAINER_HIGH
                        } else {
                            Color32::TRANSPARENT
                        };

                        painter.rect_filled(item_rect.0, ROUNDING_STADIUM, bg_color);

                        // Icon and label color
                        let fg_color = if is_active {
                            ON_PRIMARY_CONTAINER
                        } else {
                            ON_SURFACE_VARIANT
                        };

                        // Draw vector icon
                        let icon_box = Rect::from_center_size(
                            Pos2::new(item_rect.0.min.x + 28.0, item_rect.0.center().y),
                            Vec2::splat(20.0),
                        );
                        match icon_kind {
                            1 => draw_icon_dashboard(painter, icon_box, fg_color),
                            2 => draw_icon_power(painter, icon_box, fg_color),
                            3 => draw_icon_fan(painter, icon_box, fg_color),
                            4 => draw_icon_battery(painter, icon_box, fg_color, 80.0),
                            5 => draw_icon_keyboard(painter, icon_box, fg_color),
                            _ => draw_icon_display(painter, icon_box, fg_color),
                        }

                        // Text label
                        let text_pos = Pos2::new(item_rect.0.min.x + 50.0, item_rect.0.center().y - 8.0);
                        painter.text(
                            text_pos,
                            egui::Align2::LEFT_TOP,
                            title,
                            egui::FontId::proportional(14.0),
                            fg_color,
                        );

                        // Active dot pill indicator
                        if is_active {
                            let ind_center = Pos2::new(item_rect.0.max.x - 16.0, item_rect.0.center().y);
                            painter.circle_filled(ind_center, 3.5, PRIMARY);
                        }
                    }

                    // Bottom info card in rail
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        ui.add_space(8.0);

                        // Thermal mode pill badge
                        if let Some(t) = &self.state.thermal {
                            let mode_col = match t.active_mode.as_str() {
                                "quiet" => COLOR_QUIET,
                                "performance" => COLOR_PERF,
                                "custom" => COLOR_CUSTOM,
                                _ => COLOR_BALANCED,
                            };
                            let pill_frame = Frame::none()
                                .fill(SURFACE_CONTAINER)
                                .rounding(ROUNDING_STADIUM)
                                .inner_margin(Margin::symmetric(14.0, 8.0));
                            pill_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                                    ui.painter().circle_filled(dot_rect.center(), 4.0, mode_col);
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(&t.power_led_color).size(12.0).strong().color(ON_SURFACE));
                                });
                            });
                        }

                        ui.add_space(6.0);

                        // Device ID Pill
                        let dmi_frame = Frame::none()
                            .fill(SURFACE_CONTAINER)
                            .rounding(Rounding::same(14.0))
                            .inner_margin(Margin::symmetric(14.0, 10.0));
                        dmi_frame.show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&self.state.dmi.product_version).size(11.0).strong().color(ON_SURFACE));
                                ui.label(RichText::new(format!("BIOS: {} · Linux Native", self.state.dmi.bios_version)).size(10.0).color(OUTLINE_MUTED));
                            });
                        });
                    });
                });
            });

        // ==============================================================================
        // Main Content View Area
        // ==============================================================================
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(SURFACE_DIM)
                    .inner_margin(Margin::symmetric(28.0, 24.0)),
            )
            .show(ctx, |ui| {
                let mut next_toast = None;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut notify_fn = |msg: String| {
                        next_toast = Some(msg);
                    };

                    match self.current_tab {
                        AppTab::Dashboard => views::dashboard::render_dashboard(ui, &mut self.state, &mut notify_fn),
                        AppTab::CustomMode => views::power::render_power(ui, &mut self.state, &mut self.power_sliders, &mut notify_fn),
                        AppTab::Oled => views::oled::render_oled(ui, &mut self.state, &mut notify_fn),
                        AppTab::Cooling => views::cooling::render_cooling(ui, &self.state, &mut self.slider_fan_rpm, &mut notify_fn),
                        AppTab::Battery => views::battery::render_battery(ui, &self.state, &mut notify_fn),
                        AppTab::Lighting => views::lighting::render_lighting(ui, &mut self.lighting_state, &mut notify_fn),
                    }
                });

                if let Some(msg) = next_toast {
                    self.notify(msg);
                }
            });

        // ==============================================================================
        // Bottom Notification Toast
        // ==============================================================================
        if let Some((msg, time)) = &self.status_msg {
            if time.elapsed() < Duration::from_secs(3) {
                egui::TopBottomPanel::bottom("status_bar")
                    .frame(
                        Frame::none()
                            .fill(SURFACE_CONTAINER_HIGH)
                            .inner_margin(Margin::symmetric(24.0, 10.0)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 4.0, COLOR_SUCCESS);
                            ui.label(
                                RichText::new(msg)
                                    .color(PRIMARY)
                                    .size(13.0)
                                    .strong(),
                            );
                        });
                    });
            }
        }

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

// ==============================================================================
// Application Entry Point
// ==============================================================================
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Legion Space")
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Legion Space",
        options,
        Box::new(|cc| Ok(Box::new(LegionSpaceApp::new(cc)))),
    )
}
