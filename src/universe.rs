use std::cell::RefCell;

use egui::Vec2;

use crate::{app::ParticleRenderingInfo, math::distance};

const PARTICLE_BASE_SIZE: f32 = 1.0;

pub struct Universe {
    particles: Vec<RefCell<ParticleMeta>>,
    pub wrap_around_size: Vec2,
    pub drag: f32,
}

#[derive(Copy, Clone)]
pub struct ParticleMeta {
    pub position: Vec2,
    pub velocity: Vec2,
    pub mass: f32,
}

impl Universe {
    pub async fn tick(&mut self) {
        let mut to_remove = Vec::new();
        let mut to_perform_merge = Vec::new();
        for (i, particle) in self.particles.iter().enumerate() {
            let mut particle = particle.borrow_mut();
            particle.position = particle.position + particle.velocity;
            particle.position.x %= self.wrap_around_size.x;
            if particle.position.x < 0.0 {
                particle.position.x = self.size().x - particle.position.x;
            }
            particle.position.y %= self.wrap_around_size.y;
            if particle.position.y < 0.0 {
                particle.position.y = self.size().y - particle.position.y;
            }
            for (j, other_particle) in self.particles.iter().enumerate() {
                if i == j {
                    continue;
                }
                let other_particle = other_particle.borrow();
                let force = particle.gravitational_force(&other_particle);
                particle.velocity += force;
                let distance = distance(particle.position, other_particle.position);
                if distance < particle.mass * PARTICLE_BASE_SIZE {
                    if !to_remove.contains(&i) {
                        to_remove.push(j);
                        to_perform_merge.push((i, other_particle.mass, other_particle.velocity));
                    }
                }
            }
            particle.velocity *= self.drag;
        }
        for (i, mass, velocity) in to_perform_merge {
            let mut target = self.particles[i].borrow_mut();
            target.mass += mass;
            target.velocity = target.velocity + velocity * (mass / target.mass);
        }
        to_remove.sort();
        to_remove.reverse();
        for i in to_remove {
            self.particles.remove(i);
        }
    }

    pub fn get_rendering_info(&self) -> Vec<ParticleRenderingInfo> {
        self.particles
            .iter()
            .map(|particle| {
                let particle = particle.borrow();
                ParticleRenderingInfo::Blue {
                    position: particle.position,
                    size: PARTICLE_BASE_SIZE * particle.mass,
                }
            })
            .collect()
    }

    pub fn size(&self) -> Vec2 {
        self.wrap_around_size
    }

    pub fn spawn_particle(
        &mut self,
        particle: ParticleMeta,
        // particle: impl Particle + Send + Sync + 'static,
    ) {
        // self.particles.push((meta, Box::new(particle)));
        self.particles.push(RefCell::new(particle))
    }

    pub fn spawn_random_particle(&mut self) {
        self.spawn_particle(ParticleMeta {
            position: Vec2::new(
                rand::random_range(0.0..self.size().x),
                rand::random_range(0.0..self.size().y),
            ),
            velocity: Vec2::new(rand::random_range(-0.5..0.5), rand::random_range(-0.5..0.5)),
            mass: 1.0,
        });
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }
}

impl Default for Universe {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            wrap_around_size: Vec2::new(1000.0, 1000.0),
            drag: 0.999,
        }
    }
}

impl ParticleMeta {
    // this is the only function that has been mostly written by claude, the rest is mine
    fn gravitational_force(&self, other: &ParticleMeta) -> Vec2 {
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
