use macroquad::prelude::*;

use crate::config::Config;

use super::arena::Arena;
use super::ball::Ball;
use super::collision::{CollisionEvent, solve_ball_contacts, solve_wall_contacts};
use super::effects::Effects;
use super::material::MaterialKind;

#[derive(Clone, Debug)]
pub struct World {
    pub config: Config,
    pub seed: u64,
    pub balls: Vec<Ball>,
    pub arena: Arena,
    pub effects: Effects,
    pub stats: WorldStats,
    collision_events: Vec<CollisionEvent>,
}

impl World {
    pub fn random(config: Config, seed: u64) -> Self {
        let config = config.normalized();
        rand::srand(seed);
        let arena = Arena::random(&config.arena, &config.render);
        let balls = spawn_balls(&config, arena.center);
        let effects = Effects::new(&config.effects);

        let mut world = Self {
            config,
            seed,
            balls,
            arena,
            effects,
            stats: WorldStats::default(),
            collision_events: Vec::with_capacity(128),
        };
        world.refresh_energy_baseline();
        world
    }

    pub fn reset(&mut self, seed: u64) {
        let config = self.config.clone();
        *self = Self::random(config, seed);
    }

    pub fn step(&mut self, dt: f32, effects_enabled: bool) {
        self.stats.frame_collisions = 0;
        self.collision_events.clear();

        let substeps = self.config.physics.substeps.max(1);
        let sub_dt = dt / substeps as f32;

        for _ in 0..substeps {
            self.arena.update(sub_dt);

            for ball in &mut self.balls {
                ball.apply_forces(sub_dt, &self.config.physics);
                apply_vortex_drive(
                    ball,
                    self.arena.center,
                    self.config.arena.outer_radius,
                    &self.config,
                    sub_dt,
                );
                ball.integrate(sub_dt);
            }

            for _ in 0..self.config.physics.solver_iterations.max(1) {
                solve_wall_contacts(
                    &mut self.balls,
                    &self.arena,
                    &self.config.physics,
                    &mut self.collision_events,
                );
                solve_ball_contacts(
                    &mut self.balls,
                    &self.config.physics,
                    &mut self.collision_events,
                );
            }
        }

        for ball in &mut self.balls {
            ball.update_trail(self.config.spawn.trail_length);
        }

        self.stats.frame_collisions = self.collision_events.len();
        self.stats.total_collisions += self.stats.frame_collisions;

        if effects_enabled {
            self.effects
                .spawn_from_events(&self.collision_events, &self.config.effects);
        }
        self.effects.update(dt, &self.config.effects);
        self.update_energy();
    }

    pub fn clear_effects(&mut self) {
        self.effects.clear();
    }

    pub fn material_counts(&self) -> MaterialCounts {
        let mut counts = MaterialCounts::default();
        for ball in &self.balls {
            match ball.material_kind {
                MaterialKind::Rubber => counts.rubber += 1,
                MaterialKind::Steel => counts.steel += 1,
                MaterialKind::Glass => counts.glass += 1,
            }
        }
        counts
    }

    pub fn nearest_ball(&self, world_position: Vec2, max_distance: f32) -> Option<&Ball> {
        self.balls
            .iter()
            .filter_map(|ball| {
                let distance = (ball.position - world_position).length();
                (distance <= max_distance.max(ball.radius * 2.0)).then_some((ball, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(ball, _)| ball)
    }

    fn refresh_energy_baseline(&mut self) {
        self.update_energy();
        self.stats.initial_energy = self.stats.total_energy.max(1.0);
        self.stats.energy_drift = 0.0;
    }

    fn update_energy(&mut self) {
        let world_height = self.config.render.virtual_height;
        let gravity = self.config.physics.gravity;
        let mut kinetic = 0.0;
        let mut potential = 0.0;

        for ball in &self.balls {
            let translational = 0.5 * ball.mass * ball.velocity.length_squared();
            let rotational = 0.25
                * ball.mass
                * ball.radius
                * ball.radius
                * ball.angular_velocity
                * ball.angular_velocity;
            kinetic += translational + rotational;
            potential += ball.mass * gravity.abs() * (world_height - ball.position.y).max(0.0);
        }

        self.stats.kinetic_energy = kinetic;
        self.stats.potential_energy = potential;
        self.stats.total_energy = kinetic + potential;

        if self.stats.initial_energy > 0.0 {
            self.stats.energy_drift =
                (self.stats.total_energy - self.stats.initial_energy) / self.stats.initial_energy;
        }
    }
}

fn apply_vortex_drive(ball: &mut Ball, center: Vec2, outer_radius: f32, config: &Config, dt: f32) {
    let offset = ball.position - center;
    let distance = offset.length();
    if distance <= f32::EPSILON {
        return;
    }

    let radius_t = (distance / outer_radius.max(1.0)).clamp(0.0, 1.0);
    let tangent = Vec2::new(-offset.y, offset.x) / distance;
    let inward = -offset / distance;
    let edge_bias = radius_t.powf(1.6);
    let circulation = tangent * config.physics.vortex_drive * (0.35 + 0.65 * (1.0 - radius_t));
    let recenter = inward * config.physics.recenter_drive * edge_bias;

    ball.velocity += (circulation + recenter) * dt;
}

#[derive(Clone, Debug, Default)]
pub struct WorldStats {
    pub total_collisions: usize,
    pub frame_collisions: usize,
    pub kinetic_energy: f32,
    pub potential_energy: f32,
    pub total_energy: f32,
    pub initial_energy: f32,
    pub energy_drift: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MaterialCounts {
    pub rubber: usize,
    pub steel: usize,
    pub glass: usize,
}

fn spawn_balls(config: &Config, center: Vec2) -> Vec<Ball> {
    let count = rand::gen_range(config.spawn.min_balls, config.spawn.max_balls + 1);
    let mut balls = Vec::with_capacity(count);
    let columns = (count as f32).sqrt().ceil() as usize;
    let rows = count.div_ceil(columns);
    let spacing = config.spawn.max_radius * 3.6;
    let field_width = columns.saturating_sub(1) as f32 * spacing;
    let field_height = rows.saturating_sub(1) as f32 * spacing;
    let origin = center - Vec2::new(field_width * 0.5, field_height * 0.5);

    for i in 0..count {
        let column = i % columns;
        let row = i / columns;
        let jitter = Vec2::new(rand::gen_range(-8.0, 8.0), rand::gen_range(-8.0, 8.0));
        let position = origin + Vec2::new(column as f32 * spacing, row as f32 * spacing) + jitter;
        let lateral_direction = if row.is_multiple_of(2) { 1.0 } else { -1.0 };
        let row_spread = (column as f32 - columns.saturating_sub(1) as f32 * 0.5) * 12.0;
        let velocity = Vec2::new(
            lateral_direction
                * rand::gen_range(config.spawn.launch_speed_min, config.spawn.launch_speed_max)
                + row_spread,
            rand::gen_range(-24.0, 24.0),
        );
        let radius = rand::gen_range(config.spawn.min_radius, config.spawn.max_radius);
        let material_kind = MaterialKind::ALL[i % MaterialKind::ALL.len()];

        balls.push(Ball::with_params(
            position,
            velocity,
            radius,
            material_kind,
            config.spawn.trail_length,
        ));
    }

    balls
}
