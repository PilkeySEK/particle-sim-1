use std::{sync::Arc, time::Duration};

use egui::{
    Align, CentralPanel, Color32, Layout, Panel, Pos2, Rect, RichText, Sense, Slider,
    SliderClamping, Vec2,
};
use tokio::{
    sync::Mutex,
    task::block_in_place,
    time::{MissedTickBehavior, interval},
};

use crate::{
    app::tps_tracker::TpsTracker,
    universe::{Object, ObjectFlags, Universe},
};

mod tps_tracker;

const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 10.0;

pub enum ObjectRenderingInfo {
    Blue { position: Vec2, radius: f32 },
}

pub struct App {
    state: Arc<tokio::sync::Mutex<AppState>>,
    num_particles_to_spawn: u64,
    zoom_factor: f32,
    view_start: Vec2,
}

struct AppState {
    universe: Universe,
    target_tps: u64,
    tps_tracker: TpsTracker,
}

impl App {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(AppState::default()));
        tokio::spawn(tick_task(Arc::clone(&state)));
        Self {
            state,
            num_particles_to_spawn: 1,
            zoom_factor: 1.0,
            view_start: Vec2::ZERO,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            universe: Universe::default(),
            target_tps: 100,
            tps_tracker: TpsTracker::new(),
        }
    }
}

async fn tick_task(state: Arc<Mutex<AppState>>) {
    let mut tps_interval = interval(Duration::from_secs_f64(
        1.0 / state.lock().await.target_tps as f64,
    ));
    tps_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tps_interval.tick().await;
        let mut state = state.lock().await;

        let new_period = Duration::from_secs_f64(1.0 / state.target_tps as f64);
        if new_period != tps_interval.period() {
            tps_interval = interval(new_period);
            tps_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        }
        state.universe.tick().await;
        state.tps_tracker.tick();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut state = block_in_place(|| self.state.blocking_lock());
        let rendering_info = state.universe.get_rendering_info();
        Panel::left("controls-and-info").show(ui, |ui| {
            ui.label(RichText::new(format!("{} TPS", state.tps_tracker.get())));
            ui.label(RichText::new(format!("Objects: {}", rendering_info.len())));
            if ui.button("Spawn random particle").clicked() {
                for _ in 0..self.num_particles_to_spawn {
                    state.universe.spawn_random_object();
                }
            }
            if ui.button("Clear").clicked() {
                state.universe.clear();
            }
            ui.add(Slider::new(&mut state.target_tps, 1..=1000));
            ui.add(
                Slider::new(&mut self.num_particles_to_spawn, 1..=100)
                    .clamping(SliderClamping::Never),
            );
            ui.add(Slider::new(&mut state.universe.drag, 0.90..=1.0));
            ui.separator();
            ui.add(Slider::new(
                &mut state.universe.wrap_around_size.x,
                100.0..=2000.0,
            ));
            ui.add(Slider::new(
                &mut state.universe.wrap_around_size.y,
                100.0..=2000.0,
            ));
        });
        CentralPanel::default().show(ui, |ui| {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta());
            ui.allocate_ui_with_layout(Vec2::ZERO, Layout::left_to_right(Align::Min), |ui| {
                if ui.button(" + ").clicked() {
                    self.zoom_factor += 0.1;
                }
                if ui.button(" - ").clicked() {
                    self.zoom_factor -= 0.1;
                }
                ui.add(Slider::new(&mut self.zoom_factor, MIN_ZOOM..=MAX_ZOOM));
                if ui.button("R").on_hover_text("Reset Zoom").clicked() {
                    self.zoom_factor = 1.0;
                    self.view_start = Vec2::ZERO;
                }
            });
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
            let clip_rect = painter.clip_rect();
            let painter_size = Vec2::new(clip_rect.width(), clip_rect.height());
            let universe_size = state.universe.size();
            let largest_size = largest_possible_paint_size(painter_size, universe_size);
            let scale = largest_size / universe_size;
            {
                let zoom_delta = scroll_delta.y * 0.01;
                if let Some(hover_pos) = response.hover_pos() {
                    let hover_world_pos = (hover_pos - clip_rect.min) / scale / self.zoom_factor;
                    let edge_distance = hover_world_pos - self.view_start;
                    // ???
                    let _ = edge_distance;
                    // self.view_start += view_offset;
                }
                self.zoom_factor += zoom_delta;
            }
            self.view_start = self
                .view_start
                .clamp(-state.universe.size() / 2.0, state.universe.size() / 2.0);

            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let mut pos = Vec2::new(pos.x, pos.y);
                pos -= Vec2::new(clip_rect.min.x, clip_rect.min.y);
                let universe_pos = pos / scale / self.zoom_factor + self.view_start;
                state.universe.spawn_object(Object {
                    position: universe_pos,
                    velocity: Vec2::ZERO,
                    mass: 1.0,
                    flags: ObjectFlags::empty(),
                });
            }
            {
                let view_delta = response.drag_delta() / scale / self.zoom_factor;
                self.view_start -= view_delta;
            }

            painter.rect_filled(
                Rect::from_min_size(clip_rect.min, largest_size),
                0,
                Color32::BLACK,
            );

            for info in rendering_info {
                match info {
                    ObjectRenderingInfo::Blue {
                        position,
                        radius: size,
                    } => {
                        let circle_center = (position - self.view_start) * scale * self.zoom_factor
                            + Vec2::new(clip_rect.min.x, clip_rect.min.y);
                        if circle_center.x < clip_rect.min.x
                            || circle_center.y < clip_rect.min.y
                            || circle_center.x > largest_size.x + clip_rect.min.x
                            || circle_center.y > largest_size.y + clip_rect.min.y
                        {
                            // Don't draw if it's not in the view
                            continue;
                        }
                        painter.circle_filled(
                            Pos2::new(circle_center.x, circle_center.y),
                            size * scale.x * self.zoom_factor,
                            Color32::BLUE,
                        );
                    }
                }
            }
        });

        ui.request_repaint();
    }
}

const fn largest_possible_paint_size(available_size: Vec2, required_dimensions: Vec2) -> Vec2 {
    let target_aspect = required_dimensions.x / required_dimensions.y;
    let available_aspect = available_size.x / available_size.y;

    if available_aspect > target_aspect {
        Vec2::new(available_size.y * target_aspect, available_size.y)
    } else {
        Vec2::new(available_size.x, available_size.x / target_aspect)
    }
}
