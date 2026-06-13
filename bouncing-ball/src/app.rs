use macroquad::prelude::*;

use crate::config::Config;
use crate::input::ViewOptions;
use crate::render::{self, OverlayState};
use crate::sim::World;

#[derive(Clone, Debug)]
pub struct App {
    pub world: World,
    pub view: ViewOptions,
    accumulator: f32,
    paused: bool,
    step_once: bool,
    time_scale: f32,
    last_steps: usize,
    dropped_time: f32,
}

impl App {
    pub fn new(config: Config) -> Self {
        let seed = fresh_seed();
        Self {
            world: World::random(config.normalized(), seed),
            view: ViewOptions::default(),
            accumulator: 0.0,
            paused: false,
            step_once: false,
            time_scale: 1.0,
            last_steps: 0,
            dropped_time: 0.0,
        }
    }

    pub fn handle_input(&mut self) {
        if is_key_pressed(KeyCode::R) {
            self.world.reset(fresh_seed());
            self.accumulator = 0.0;
            self.dropped_time = 0.0;
        }

        if is_key_pressed(KeyCode::H) || is_key_pressed(KeyCode::F1) {
            self.view.help_overlay = !self.view.help_overlay;
        }

        if is_key_pressed(KeyCode::Tab) {
            self.view.compact_hud = !self.view.compact_hud;
        }

        if is_key_pressed(KeyCode::P) {
            self.paused = !self.paused;
        }

        if is_key_pressed(KeyCode::Period) || is_key_pressed(KeyCode::Right) {
            self.step_once = true;
            self.paused = true;
        }

        if is_key_pressed(KeyCode::Key1) {
            self.time_scale = 1.0;
        } else if is_key_pressed(KeyCode::Key2) {
            self.time_scale = 0.5;
        } else if is_key_pressed(KeyCode::Key3) {
            self.time_scale = 0.25;
        }

        if is_key_pressed(KeyCode::V) {
            self.view.velocity_vectors = !self.view.velocity_vectors;
        }
        if is_key_pressed(KeyCode::N) {
            self.view.collision_normals = !self.view.collision_normals;
        }
        if is_key_pressed(KeyCode::T) {
            self.view.trails = !self.view.trails;
        }
        if is_key_pressed(KeyCode::E) {
            self.view.effects = !self.view.effects;
            if !self.view.effects {
                self.world.clear_effects();
            }
        }
    }

    pub fn update(&mut self, raw_frame_dt: f32) {
        self.last_steps = 0;
        self.dropped_time = 0.0;

        if self.step_once {
            self.world
                .step(self.world.config.physics.fixed_dt, self.view.effects);
            self.last_steps = 1;
            self.step_once = false;
            return;
        }

        if self.paused {
            self.world
                .effects
                .update(raw_frame_dt, &self.world.config.effects);
            return;
        }

        let fixed_dt = self.world.config.physics.fixed_dt;
        let max_frame_time = self.world.config.physics.max_frame_time;
        let max_steps_per_frame = self.world.config.physics.max_steps_per_frame;
        let frame_dt = raw_frame_dt.min(max_frame_time) * self.time_scale;
        self.accumulator += frame_dt;

        let max_accumulated = fixed_dt * max_steps_per_frame as f32;
        if self.accumulator > max_accumulated {
            self.dropped_time = self.accumulator - max_accumulated;
            self.accumulator = max_accumulated;
        }

        while self.accumulator >= fixed_dt && self.last_steps < max_steps_per_frame {
            self.world.step(fixed_dt, self.view.effects);
            self.accumulator -= fixed_dt;
            self.last_steps += 1;
        }
    }

    pub fn draw(&self) {
        render::draw(
            &self.world,
            &self.view,
            OverlayState {
                fps: get_fps(),
                fixed_hz: (1.0 / self.world.config.physics.fixed_dt).round() as u32,
                substeps: self.world.config.physics.substeps,
                last_steps: self.last_steps,
                dropped_time: self.dropped_time,
                time_scale: self.time_scale,
                paused: self.paused,
            },
        );
    }
}

fn fresh_seed() -> u64 {
    let now = macroquad::miniquad::date::now();
    let bits = now.to_bits();
    bits ^ bits.rotate_left(17) ^ 0x9E37_79B9_7F4A_7C15
}
