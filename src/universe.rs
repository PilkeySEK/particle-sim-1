use std::f32::consts::PI;

use bitflags::bitflags;
use egui::Vec2;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use tokio::sync::mpsc::unbounded_channel;

use crate::{app::ObjectRenderingInfo, math::distance};

pub struct Universe {
    objects: Vec<Object>, // Vec<std::sync::Mutex<Object>>,
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
        struct ObjectChange {
            /// The index of the object
            index: usize,
            /// Add this velocity
            velocity: Vec2,
            /// Add this position
            position: Vec2,
            /// OR these flags
            flags: ObjectFlags,
            /// Add this mass
            mass: f32,
        }
        #[expect(non_local_definitions)]
        impl Object {
            fn diff(&self, other: &Object, index: usize) -> ObjectChange {
                ObjectChange {
                    index,
                    velocity: other.velocity - self.velocity,
                    position: other.position - self.position,
                    flags: other.flags.difference(other.flags),
                    mass: self.mass - other.mass,
                }
            }
        }

        // let mut to_remove = Vec::new();
        // let (to_merge_tx, mut to_merge_rx) = unbounded_channel();
        let (obj_changes_tx, mut obj_changes_rx) = unbounded_channel();
        let changes = &obj_changes_tx;
        self.objects.par_iter().enumerate().for_each(|(i, obj)| {
            let original_obj = obj;
            let mut obj = *obj;
            obj.position = obj.position + obj.velocity;
            obj.position.x %= self.wrap_around_size.x;
            if obj.position.x < 0.0 {
                obj.position.x = self.size().x - obj.position.x;
            }
            obj.position.y %= self.wrap_around_size.y;
            if obj.position.y < 0.0 {
                obj.position.y = self.size().y - obj.position.y;
            }
            for (j, other_obj) in self.objects.iter().enumerate() {
                if i == j {
                    continue;
                }
                let original_obj = other_obj;
                let mut other_obj = *other_obj;
                let force = obj.gravitational_force(&other_obj);
                obj.velocity += force;
                let distance = distance(obj.position, other_obj.position);
                if distance < obj.radius() * 1.25 {
                    if !obj.flags.contains(ObjectFlags::REMOVE_NEXT) {
                        other_obj.flags |= ObjectFlags::REMOVE_NEXT;
                        changes
                            .send(ObjectChange {
                                index: i,
                                velocity: other_obj.velocity,
                                position: Vec2::ZERO,
                                flags: ObjectFlags::REMOVE_NEXT,
                                mass: other_obj.mass,
                            })
                            .unwrap();
                        /*to_merge_tx
                        .send((i, other_obj.mass, other_obj.velocity))
                        .unwrap();*/
                    }
                }
                changes.send(original_obj.diff(&other_obj, j)).unwrap();
                // *other_obj_mutex.lock().unwrap() = other_obj;
            }
            obj.velocity *= self.drag;

            changes.send(original_obj.diff(&obj, i)).unwrap();
            // *obj_mutex.lock().unwrap() = obj;
        });
        // drop(to_merge_tx);
        /*while let Some((i, mass, velocity)) = to_merge_rx.recv().await {
            let mut target = self.objects[i].lock().unwrap();
            target.mass += mass;
            target.velocity = target.velocity + velocity * (mass / target.mass);
        }*/
        drop(obj_changes_tx);
        while let Some(changes) = obj_changes_rx.recv().await {
            let obj = &mut self.objects[changes.index];
            obj.velocity += changes.velocity;
            obj.position += changes.position;
            obj.mass += changes.mass;
            obj.flags |= changes.flags;
        }
        self.objects
            .retain(|obj| !obj.flags.contains(ObjectFlags::REMOVE_NEXT));
        // to_remove.sort();
        // to_remove.reverse();
        // for i in to_remove {
        //     self.objects.remove(i);
        // }
    }

    pub fn get_rendering_info(&self) -> Vec<ObjectRenderingInfo> {
        self.objects
            .iter()
            .map(|object| ObjectRenderingInfo::Object {
                position: object.position,
                radius: object.radius(),
            })
            .collect()
    }

    pub fn size(&self) -> Vec2 {
        self.wrap_around_size
    }

    pub fn spawn_object(
        &mut self,
        object: Object,
        // particle: impl Particle + Send + Sync + 'static,
    ) {
        // self.particles.push((meta, Box::new(particle)));
        self.objects.push(object)
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
        const G: f32 = 0.5;
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
