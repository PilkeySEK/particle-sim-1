use std::{sync::Arc, time::Duration};

use egui::{CentralPanel, Color32, Panel, Pos2, Rect, RichText, Slider, Vec2};
use tokio::{sync::Mutex, task::block_in_place, time::interval};

use crate::{app::tps_tracker::TpsTracker, universe::Universe};

mod tps_tracker;

pub enum ParticleRenderingInfo {
    Blue { position: Vec2, size: f32 },
}

pub struct App {
    state: Arc<tokio::sync::Mutex<AppState>>,
    num_particles_to_spawn: u64,
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
    loop {
        tps_interval.tick().await;
        let mut state = state.lock().await;

        let new_period = Duration::from_secs_f64(1.0 / state.target_tps as f64);
        if new_period != tps_interval.period() {
            tps_interval = interval(new_period);
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
                    state.universe.spawn_random_particle();
                }
            }
            if ui.button("Clear").clicked() {
                state.universe.clear();
            }
            ui.add(Slider::new(&mut state.target_tps, 1..=1000));
            ui.add(Slider::new(&mut self.num_particles_to_spawn, 1..=100));
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
            let painter = ui.painter();
            let clip_rect = painter.clip_rect();
            let painter_size = Vec2::new(clip_rect.width(), clip_rect.height());
            let universe_size = state.universe.size();
            let largest_size = largest_possible_paint_size(painter_size, universe_size);
            let scale = largest_size / universe_size;

            painter.rect_filled(
                Rect::from_min_size(clip_rect.min, largest_size),
                0,
                Color32::BLACK,
            );

            for info in rendering_info {
                match info {
                    ParticleRenderingInfo::Blue { position, size } => {
                        painter.circle_filled(
                            {
                                let vec2 = (position * scale)
                                    + Vec2::new(clip_rect.min.x, clip_rect.min.y);
                                Pos2::new(vec2.x, vec2.y)
                            },
                            size * scale.x,
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
