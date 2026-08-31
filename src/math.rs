use egui::Vec2;

pub fn distance(a: Vec2, b: Vec2) -> f32 {
    ((a.x - b.x).powf(2.0) + (a.y - b.y).powf(2.0)).sqrt()
}
