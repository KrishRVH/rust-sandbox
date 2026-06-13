use std::collections::VecDeque;
use std::f32::consts::PI;

use macroquad::prelude::*;

use crate::config::{PhysicsConfig, SpawnConfig};

use super::material::MaterialKind;

#[derive(Clone, Debug)]
pub struct Ball {
    pub position: Vec2,
    pub previous_position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
    pub mass: f32,
    pub inverse_mass: f32,
    pub rotation: f32,
    pub angular_velocity: f32,
    pub material_kind: MaterialKind,
    pub heat: f32,
    pub trail: VecDeque<Vec2>,
}

impl Ball {
    pub fn with_params(
        position: Vec2,
        velocity: Vec2,
        radius: f32,
        material_kind: MaterialKind,
        trail_length: usize,
    ) -> Self {
        let material = material_kind.material();
        let mass = material.density * PI * radius * radius / 115.0;
        let inverse_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };

        let mut trail = VecDeque::with_capacity(trail_length);
        trail.push_back(position);

        Self {
            position,
            previous_position: position,
            velocity,
            radius,
            mass,
            inverse_mass,
            rotation: 0.0,
            angular_velocity: rand::gen_range(-5.0, 5.0),
            material_kind,
            heat: 0.0,
            trail,
        }
    }

    pub fn random(position: Vec2, spawn: &SpawnConfig) -> Self {
        let radius = rand::gen_range(spawn.min_radius, spawn.max_radius);
        let material_kind = MaterialKind::ALL[rand::gen_range(0, MaterialKind::ALL.len())];
        let launch_angle: f32 = rand::gen_range(-2.75, -0.35);
        let launch_speed = rand::gen_range(spawn.launch_speed_min, spawn.launch_speed_max);
        let velocity = Vec2::new(launch_angle.cos(), launch_angle.sin()) * launch_speed
            + Vec2::new(rand::gen_range(-80.0, 80.0), rand::gen_range(-40.0, 40.0));

        Self::with_params(
            position,
            velocity,
            radius,
            material_kind,
            spawn.trail_length,
        )
    }

    pub fn material(&self) -> super::material::Material {
        self.material_kind.material()
    }

    pub fn inverse_inertia(&self) -> f32 {
        let inertia = 0.5 * self.mass * self.radius * self.radius;
        if inertia > 0.0 { 1.0 / inertia } else { 0.0 }
    }

    pub fn apply_forces(&mut self, dt: f32, physics: &PhysicsConfig) {
        self.velocity.y += physics.gravity * dt;

        let speed_sq = self.velocity.length_squared();
        if speed_sq > 0.01 {
            let speed = speed_sq.sqrt();
            let drag_force = 0.5
                * physics.air_density
                * speed_sq
                * self.material().drag_coefficient
                * self.radius;
            let drag_accel = drag_force * self.inverse_mass;
            self.velocity -= self.velocity / speed * drag_accel * dt;
        }

        self.angular_velocity *= (-physics.angular_damping_per_sec * dt).exp();
        self.heat *= (-physics.heat_decay_per_sec * dt).exp();
        if self.heat < 0.001 {
            self.heat = 0.0;
        }
    }

    pub fn integrate(&mut self, dt: f32) {
        self.previous_position = self.position;
        self.position += self.velocity * dt;
        self.rotation += self.angular_velocity * dt;
    }

    pub fn add_heat_from_impulse(&mut self, impulse: f32) {
        self.heat = (self.heat + impulse.abs() * 0.0035).min(1.0);
    }

    pub fn update_trail(&mut self, max_len: usize) {
        if max_len == 0 {
            self.trail.clear();
            return;
        }

        while self.trail.len() >= max_len {
            self.trail.pop_front();
        }
        self.trail.push_back(self.position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_trail_handles_zero_length_without_looping() {
        let mut ball = Ball::with_params(Vec2::ZERO, Vec2::ZERO, 10.0, MaterialKind::Rubber, 4);
        ball.update_trail(0);
        assert!(ball.trail.is_empty());
    }
}
