use std::cell::RefCell;

use egui::Vec2;

use crate::{app::ObjectRenderingInfo, math::distance};

const OBJECT_BASE_SIZE: f32 = 1.0;

pub struct Universe {
    objects: Vec<RefCell<Object>>,
    pub wrap_around_size: Vec2,
    pub drag: f32,
}

#[derive(Copy, Clone)]
pub struct Object {
    pub position: Vec2,
    pub velocity: Vec2,
    pub mass: f32,
}

impl Universe {
    pub async fn tick(&mut self) {
        let mut to_remove = Vec::new();
        let mut to_perform_merge = Vec::new();
        for (i, object) in self.objects.iter().enumerate() {
            let mut object = object.borrow_mut();
            object.position = object.position + object.velocity;
            object.position.x %= self.wrap_around_size.x;
            if object.position.x < 0.0 {
                object.position.x = self.size().x - object.position.x;
            }
            object.position.y %= self.wrap_around_size.y;
            if object.position.y < 0.0 {
                object.position.y = self.size().y - object.position.y;
            }
            for (j, other_object) in self.objects.iter().enumerate() {
                if i == j {
                    continue;
                }
                let other_object = other_object.borrow();
                let force = object.gravitational_force(&other_object);
                object.velocity += force;
                let distance = distance(object.position, other_object.position);
                if distance < object.mass * OBJECT_BASE_SIZE {
                    if !to_remove.contains(&i) {
                        to_remove.push(j);
                        to_perform_merge.push((i, other_object.mass, other_object.velocity));
                    }
                }
            }
            object.velocity *= self.drag;
        }
        for (i, mass, velocity) in to_perform_merge {
            let mut target = self.objects[i].borrow_mut();
            target.mass += mass;
            target.velocity = target.velocity + velocity * (mass / target.mass);
        }
        to_remove.sort();
        to_remove.reverse();
        for i in to_remove {
            self.objects.remove(i);
        }
    }

    pub fn get_rendering_info(&self) -> Vec<ObjectRenderingInfo> {
        self.objects
            .iter()
            .map(|object| {
                let object = object.borrow();
                ObjectRenderingInfo::Blue {
                    position: object.position,
                    size: OBJECT_BASE_SIZE * object.mass,
                }
            })
            .collect()
    }

    pub fn size(&self) -> Vec2 {
        self.wrap_around_size
    }

    pub fn spawn_object(
        &mut self,
        object: Object,
        // object: impl object + Send + Sync + 'static,
    ) {
        // self.objects.push((meta, Box::new(object)));
        self.objects.push(RefCell::new(object))
    }

    pub fn spawn_random_object(&mut self) {
        self.spawn_object(Object {
            position: Vec2::new(
                rand::random_range(0.0..self.size().x),
                rand::random_range(0.0..self.size().y),
            ),
            velocity: Vec2::new(rand::random_range(-0.5..0.5), rand::random_range(-0.5..0.5)),
            mass: 1.0,
        });
    }

    pub fn clear(&mut self) {
        self.objects.clear();
    }
}

impl Default for Universe {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            wrap_around_size: Vec2::new(1000.0, 1000.0),
            drag: 0.999,
        }
    }
}

impl Object {
    // this is the only function that has been mostly written by claude, the rest is mine
    fn gravitational_force(&self, other: &Object) -> Vec2 {
        const G: f32 = 1.0;

        let delta = other.position - self.position;
        let distance_sq = delta.x * delta.x + delta.y * delta.y;

        // Avoid division by zero
        let distance_sq = distance_sq.max(1e-6);
        let distance = distance_sq.sqrt();

        let force_magnitude = G * (self.mass * other.mass) / distance_sq;
        let direction = delta * (1.0 / distance);

        direction * force_magnitude
    }
}
