use std::{f32::consts::PI, sync::Mutex};

use bitflags::bitflags;
use egui::Vec2;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use tokio::sync::mpsc::unbounded_channel;

use crate::{app::ObjectRenderingInfo, math::distance, util::Immutable};

pub struct Universe {
    pub objects: Vec<std::sync::Mutex<Object>>,
    pub wrap_around_size: Vec2,
    pub drag: f32,
}

#[derive(Copy, Clone)]
pub enum ObjectKind {
    Fixed { pos: Immutable<Vec2> },
    NotFixed { pos: Vec2, remove_next: bool },
}

#[derive(Copy, Clone)]
pub struct Object {
    // /// `Immutable` if the object is fixed (cannot be moved).
    // pub position: Either<Vec2, Immutable<Vec2>>,
    pub kind: ObjectKind,
    pub velocity: Vec2,
    pub mass: f32,
    // pub flags: ObjectFlags,
}

impl Object {
    pub fn radius(&self) -> f32 {
        (self.mass / PI).sqrt()
    }

    pub fn pos(&self) -> Vec2 {
        match self.kind {
            ObjectKind::Fixed { pos } => *pos,
            ObjectKind::NotFixed { pos, .. } => pos,
        }
    }

    pub fn remove_next(&self) -> bool {
        matches!(
            self.kind,
            ObjectKind::NotFixed {
                remove_next: true,
                ..
            }
        )
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
                if let ObjectKind::NotFixed { pos, .. } = &mut obj.kind {
                    *pos = *pos + obj.velocity;
                }
                if let ObjectKind::NotFixed { pos, .. } = &mut obj.kind {
                    pos.x %= self.wrap_around_size.x;
                    if pos.x < 0.0 {
                        pos.x = self.size().x - pos.x;
                    }
                    pos.y %= self.wrap_around_size.y;
                    if pos.y < 0.0 {
                        pos.y = self.size().y - pos.y;
                    }
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
                    let distance = distance(other_obj.pos(), obj.pos());
                    if distance < obj.radius() * 1.25 {
                        match (&mut obj.kind, &mut other_obj.kind) {
                            (ObjectKind::Fixed { .. }, ObjectKind::Fixed { .. }) => {}
                            (
                                ObjectKind::NotFixed {
                                    remove_next: remove_next_1,
                                    ..
                                },
                                ObjectKind::NotFixed {
                                    remove_next: remove_next_2,
                                    ..
                                },
                            ) => {
                                if !*remove_next_1 && !*remove_next_2 {
                                    *remove_next_2 = true;
                                    to_merge_tx
                                        .send((i, other_obj.mass, other_obj.velocity))
                                        .unwrap();
                                }
                            }
                            (
                                ObjectKind::Fixed { .. },
                                ObjectKind::NotFixed { remove_next, .. },
                            ) => {
                                if !*remove_next {
                                    *remove_next = true;
                                    to_merge_tx
                                        .send((i, other_obj.mass, other_obj.velocity))
                                        .unwrap();
                                }
                            }
                            (
                                ObjectKind::NotFixed { remove_next, .. },
                                ObjectKind::Fixed { .. },
                            ) => {
                                if !*remove_next {
                                    *remove_next = true;
                                    to_merge_tx.send((j, obj.mass, obj.velocity)).unwrap();
                                }
                            } /*
                              (true, ObjectKind::NotFixed { .. }) => {}
                              (_, false) => {
                                  obj.flags &= !ObjectFlags::REMOVE_NEXT;
                                  if other_obj.position.is_left() {
                                      other_obj.flags |= ObjectFlags::REMOVE_NEXT;

                                      to_merge_tx
                                          .send((i, other_obj.mass, other_obj.velocity))
                                          .unwrap();
                                  }
                              }
                              (false, true) => {
                                  if !other_obj.flags.contains(ObjectFlags::REMOVE_NEXT) {
                                      obj.flags |= ObjectFlags::REMOVE_NEXT;

                                      to_merge_tx.send((i, other_obj.mass, other_obj.velocity)).unwrap();
                                  }
                              }
                              */
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
            .retain(|obj| !obj.lock().unwrap().remove_next());
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
                ObjectRenderingInfo::Object {
                    position: object.pos(),
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
            kind: ObjectKind::NotFixed {
                pos: Vec2::new(
                    rand::random_range(0.0..self.size().x),
                    rand::random_range(0.0..self.size().y),
                ),
                remove_next: false,
            },
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
            drag: 1.0, //0.999,
        }
    }
}

impl Object {
    // this is the only function that has been mostly written by claude, the rest is mine
    fn gravitational_force(&self, other: &Object) -> Vec2 {
        const G: f32 = 0.5;
        const FORCE_LIMIT: Vec2 = Vec2::new(100.0, 100.0);

        let delta = other.pos() - self.pos();
        let distance_sq = delta.x * delta.x + delta.y * delta.y;

        // Avoid division by zero
        let distance_sq = distance_sq.max(1e-6);
        let distance = distance_sq.sqrt();

        let force_magnitude = G * (self.mass * other.mass) / distance_sq;
        let direction = delta * (1.0 / distance);

        (direction * force_magnitude).clamp(-FORCE_LIMIT, FORCE_LIMIT)
    }
}
