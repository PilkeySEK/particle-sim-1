//! Barnes-Hut

use egui::Vec2;

use crate::universe::Object;

pub struct BarnesHutObjectStore {
    root: BHNode,
    pub size: Vec2,
}

struct BHNode {
    children: Box<[Option<BHNode>; 4]>,
    object: Option<Object>,
}

impl BarnesHutObjectStore {
    pub fn add_object(&mut self, obj: Object) {
        let pos = obj.position;
    }
}