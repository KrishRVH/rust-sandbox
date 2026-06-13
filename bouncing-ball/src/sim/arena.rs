use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use crate::config::{ArenaConfig, RenderConfig};
use crate::geometry::rotate_point_sincos;

#[derive(Clone, Debug)]
pub struct Arena {
    pub center: Vec2,
    pub layers: Vec<ArenaLayer>,
}

impl Arena {
    pub fn random(arena_config: &ArenaConfig, render_config: &RenderConfig) -> Self {
        let center = render_config.world_center();
        let layer_count = rand::gen_range(arena_config.min_layers, arena_config.max_layers + 1);
        let mut layers = Vec::with_capacity(layer_count);

        for index in 0..layer_count {
            let radius = arena_config.outer_radius - index as f32 * arena_config.layer_spacing;
            layers.push(ArenaLayer::random(
                center,
                radius,
                index,
                layer_count,
                arena_config,
            ));
        }

        Self { center, layers }
    }

    pub fn update(&mut self, dt: f32) {
        for layer in &mut self.layers {
            layer.update(dt);
        }
    }

    pub fn outer(&self) -> Option<&ArenaLayer> {
        self.layers.first()
    }
}

#[derive(Clone, Debug)]
pub struct ArenaLayer {
    pub center: Vec2,
    pub radius: f32,
    pub base_vertices: Vec<Vec2>,
    pub world_vertices: Vec<Vec2>,
    pub active_edges: Vec<bool>,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub index: usize,
}

impl ArenaLayer {
    fn random(
        center: Vec2,
        radius: f32,
        index: usize,
        _total_layers: usize,
        config: &ArenaConfig,
    ) -> Self {
        let sides = config.sides_per_layer.max(3);
        let mut base_vertices = Vec::with_capacity(sides);
        for i in 0..sides {
            let angle = i as f32 * TAU / sides as f32 - PI / sides as f32;
            base_vertices.push(Vec2::new(angle.cos(), angle.sin()) * radius);
        }

        let mut active_edges = vec![true; sides];
        if index > 0 {
            let max_removed = (sides / 3).max(2);
            let edges_to_remove = rand::gen_range(2, max_removed + 1);
            let mut removed = 0;
            while removed < edges_to_remove {
                let candidate = rand::gen_range(0, sides);
                let prev = if candidate == 0 {
                    sides - 1
                } else {
                    candidate - 1
                };
                let next = (candidate + 1) % sides;
                if active_edges[candidate] && active_edges[prev] && active_edges[next] {
                    active_edges[candidate] = false;
                    removed += 1;
                }
            }
        }

        let direction = if rand::gen_range(0, 2) == 0 {
            -1.0
        } else {
            1.0
        };
        let speed_variation = rand::gen_range(0.55, 1.35);
        let depth_scale = 1.0 + index as f32 * 0.18;
        let rotation_speed = config.rotation_speed_base * speed_variation * depth_scale * direction;

        let mut layer = Self {
            center,
            radius,
            base_vertices,
            world_vertices: Vec::with_capacity(sides),
            active_edges,
            rotation: rand::gen_range(0.0, TAU),
            rotation_speed,
            index,
        };
        layer.rebuild_world_vertices();
        layer
    }

    pub fn update(&mut self, dt: f32) {
        self.rotation = (self.rotation + self.rotation_speed * dt) % TAU;
        self.rebuild_world_vertices();
    }

    pub fn rebuild_world_vertices(&mut self) {
        let (sin, cos) = self.rotation.sin_cos();
        self.world_vertices.clear();
        self.world_vertices.extend(
            self.base_vertices
                .iter()
                .map(|&vertex| rotate_point_sincos(vertex, sin, cos) + self.center),
        );
    }

    pub fn active_segment(&self, edge_index: usize) -> Option<(Vec2, Vec2)> {
        if !self.active_edges[edge_index] {
            return None;
        }
        let start = self.world_vertices[edge_index];
        let end = self.world_vertices[(edge_index + 1) % self.world_vertices.len()];
        Some((start, end))
    }
}
