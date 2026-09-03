use std::{f32::consts::PI, sync::Mutex};

use bitflags::bitflags;
use egui::Vec2;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use tokio::sync::mpsc::unbounded_channel;

use crate::{app::ObjectRenderingInfo, math::distance};

pub struct Universe {
    objects: Vec<std::sync::Mutex<Object>>,
    pub wrap_around_size: Vec2,
    pub drag: f32,
}

#[derive(Copy, Clone)]
pub struct Object {
    pub position: Vec2,
    pub velocity: Vec2,
    pub mass: f32,
    pub flags: ObjectFlags,
}

impl Object {
    pub fn radius(&self) -> f32 {
        (self.mass / PI).sqrt()
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct ObjectFlags: u8 {
        const REMOVE_NEXT = 1 << 0;
    }
}

impl Universe {
    pub async fn tick(&mut self) {
        // let mut to_remove = Vec::new();
        let (to_merge_tx, mut to_merge_rx) = unbounded_channel();
        self.objects
            .par_iter()
            .enumerate()
            .for_each(|(i, obj_mutex)| {
                let obj_guard = obj_mutex.lock().unwrap();
                let mut obj = obj_guard.clone();
                drop(obj_guard);
                obj.position = obj.position + obj.velocity;
                obj.position.x %= self.wrap_around_size.x;
                if obj.position.x < 0.0 {
                    obj.position.x = self.size().x - obj.position.x;
                }
                obj.position.y %= self.wrap_around_size.y;
                if obj.position.y < 0.0 {
                    obj.position.y = self.size().y - obj.position.y;
                }
                for (j, other_obj_mutex) in self.objects.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    let other_obj_guard = other_obj_mutex.lock().unwrap();
                    let mut other_obj = other_obj_guard.clone();
                    drop(other_obj_guard);
                    let force = obj.gravitational_force(&other_obj);
                    obj.velocity += force;
                    let distance = distance(obj.position, other_obj.position);
                    if distance < obj.radius() * 1.25 {
                        if !obj.flags.contains(ObjectFlags::REMOVE_NEXT) {
                            other_obj.flags |= ObjectFlags::REMOVE_NEXT;
                            to_merge_tx
                                .send((i, other_obj.mass, other_obj.velocity))
                                .unwrap();
                        }
                    }
                    *other_obj_mutex.lock().unwrap() = other_obj;
                }
                obj.velocity *= self.drag;
                *obj_mutex.lock().unwrap() = obj;
            });
        drop(to_merge_tx);
        while let Some((i, mass, velocity)) = to_merge_rx.recv().await {
            let mut target = self.objects[i].lock().unwrap();
            target.mass += mass;
            target.velocity = target.velocity + velocity * (mass / target.mass);
        }
        self.objects
            .retain(|obj| !obj.lock().unwrap().flags.contains(ObjectFlags::REMOVE_NEXT));
        // to_remove.sort();
        // to_remove.reverse();
        // for i in to_remove {
        //     self.objects.remove(i);
        // }
    }

    pub fn get_rendering_info(&self) -> Vec<ObjectRenderingInfo> {
        self.objects
            .iter()
            .map(|object| {
                let object = object.lock().unwrap();
                ObjectRenderingInfo::Blue {
                    position: object.position,
                    radius: object.radius(),
                }
            })
            .collect()
    }

    pub fn size(&self) -> Vec2 {
        self.wrap_around_size
    }

    pub fn spawn_object(
        &mut self,
        particle: Object,
        // particle: impl Particle + Send + Sync + 'static,
    ) {
        // self.particles.push((meta, Box::new(particle)));
        self.objects.push(Mutex::new(particle))
    }

    pub fn spawn_random_object(&mut self) {
        self.spawn_object(Object {
            position: Vec2::new(
                rand::random_range(0.0..self.size().x),
                rand::random_range(0.0..self.size().y),
            ),
            velocity: Vec2::new(rand::random_range(-0.5..0.5), rand::random_range(-0.5..0.5)),
            mass: 1.0,
            flags: ObjectFlags::empty(),
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
        const G: f32 = 0.1;
        const FORCE_LIMIT: Vec2 = Vec2::new(100.0, 100.0);

        let delta = other.position - self.position;
        let distance_sq = delta.x * delta.x + delta.y * delta.y;

        // Avoid division by zero
        let distance_sq = distance_sq.max(1e-6);
        let distance = distance_sq.sqrt();

        let force_magnitude = G * (self.mass * other.mass) / distance_sq;
        let direction = delta * (1.0 / distance);

        (direction * force_magnitude).clamp(-FORCE_LIMIT, FORCE_LIMIT)
    }
}
