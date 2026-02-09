#![forbid(unsafe_code)]

//! Advanced Bouncing Ball Simulation using macroquad.
//!
//! This demo focuses on being:
//! - **Readable teaching code** (explicit types, small functions, comments about intent).
//! - **Borrow-checker friendly** (no `unsafe`, clean `split_at_mut` for pairwise collisions).
//! - **Game-loop friendly** (avoids per-frame allocations in hot paths).
//!
//! Notes:
//! - Units are in “pixels” and “seconds”.
//! - The physics is *plausible*, not a full rigid-body engine.

use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

// ============================================================================
// GLOBAL CONSTANTS
// ============================================================================

/// Maximum number of historical positions stored per ball for trail rendering.
const TRAIL_LENGTH_MAX: usize = 15;

/// Clamp for large frame times, in seconds.
///
/// We still use a fixed timestep internally; this just prevents the
/// “spiral of death” after a breakpoint or long stall.
const MAX_FRAME_TIME: f32 = 0.25;

/// Fixed physics step, in seconds.
const FIXED_DT: f32 = 1.0 / 120.0;

/// Hard cap on how many fixed steps we run per rendered frame.
const MAX_STEPS_PER_FRAME: usize = 10;

/// Maximum time we allow to accumulate for catch-up.
///
/// This is tied to `MAX_STEPS_PER_FRAME` so we never build up more backlog
/// than we are willing to process.
const MAX_ACCUMULATED_TIME: f32 = FIXED_DT * MAX_STEPS_PER_FRAME as f32;

// ============================================================================
// PHYSICS & SIMULATION CONSTANTS
// ============================================================================

/// Downward acceleration in “pixels per second squared”.
const GRAVITY: f32 = 800.0;

/// Smallest ball radius.
const MIN_BALL_RADIUS: f32 = 4.0;

/// Largest ball radius.
const MAX_BALL_RADIUS: f32 = 10.0;

/// Minimum linear speed a ball is allowed to have after a collision.
///
/// This prevents balls from “sticking” to surfaces.
const MIN_VELOCITY: f32 = 50.0;

/// Number of physics substeps *inside a fixed step*.
///
/// More substeps ⇒ more accurate wall collisions for fast balls.
const SUBSTEPS: usize = 8;

/// Simple air density parameter for drag computation.
///
/// This is *not* physically accurate; it's just a tunable “air thickness”.
const AIR_DENSITY: f32 = 0.001;

/// Rotational damping applied each second (toy model).
const ANGULAR_DAMPING_PER_SEC: f32 = 2.0;

/// How quickly ball temperature normalizes back toward 1.0.
///
/// Higher values ⇒ slower cooling.
const TEMPERATURE_DECAY: f32 = 0.98;

// ============================================================================
// MATERIAL SYSTEM
// ============================================================================

/// Logical identifier for a material type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaterialKind {
    Rubber,
    Steel,
    Glass,
}

/// Physical-ish properties of a material.
#[derive(Clone, Copy, Debug)]
struct Material {
    kind: MaterialKind,
    /// Density (arbitrary units) – used to approximate mass.
    density: f32,
    /// Coefficient of restitution (bounciness).
    ///
    /// 0.0 = perfectly inelastic; 1.0 = perfectly elastic.
    restitution: f32,
    /// Surface friction coefficient.
    friction: f32,
    /// Shape factor for drag.
    drag_coefficient: f32,
    /// Base RGB color (0.0–1.0 each) used as starting point for rendering.
    color_base: [f32; 3],
}

/// Canonical material definitions.
const MATERIAL_RUBBER: Material = Material {
    kind: MaterialKind::Rubber,
    density: 1.0,
    restitution: 0.85,
    friction: 0.8,
    drag_coefficient: 0.47,
    color_base: [0.8, 0.3, 0.3],
};

const MATERIAL_STEEL: Material = Material {
    kind: MaterialKind::Steel,
    density: 3.0,
    restitution: 0.6,
    friction: 0.4,
    drag_coefficient: 0.4,
    color_base: [0.7, 0.7, 0.8],
};

const MATERIAL_GLASS: Material = Material {
    kind: MaterialKind::Glass,
    density: 2.0,
    restitution: 0.95,
    friction: 0.2,
    drag_coefficient: 0.45,
    color_base: [0.6, 0.8, 0.9],
};

/// Treat the arena walls as an extremely heavy material.
const WALL_MATERIAL: Material = Material {
    kind: MaterialKind::Steel,
    density: 10.0,
    restitution: 0.8,
    friction: 0.5,
    drag_coefficient: 0.0,
    color_base: [0.8, 0.8, 0.8],
};

// ============================================================================
// VISUAL EFFECT CONSTANTS
// ============================================================================

/// Base speed at which ripples expand (pixels per second).
const RIPPLE_BASE_SPEED: f32 = 150.0;

/// Minimum radius for a ripple.
const MIN_RIPPLE_RADIUS: f32 = 20.0;

/// Maximum radius for a ripple.
const MAX_RIPPLE_RADIUS: f32 = 300.0;

/// Speed at which sound wave rings expand (pixels per second).
const SOUND_WAVE_SPEED: f32 = 340.0;

/// Length of the spin indicator line, as a multiple of ball radius.
const SPIN_INDICATOR_LENGTH: f32 = 1.5;

/// Segment count for ring-like effects.
const RING_SEGMENTS: usize = 60;

// ============================================================================
// ARENA CONFIGURATION
// ============================================================================

const MIN_BALLS: usize = 3;
const MAX_BALLS: usize = 10;

const MIN_LAYERS: usize = 2;
const MAX_LAYERS: usize = 4;

/// Distance (in pixels) between successive polygon layers.
const LAYER_SPACING: f32 = 70.0;

/// Base angular speed of layers (radians per second).
const ROTATION_SPEED_BASE: f32 = 0.3;

/// Number of sides per polygon layer (10 = decagon).
const SIDES_PER_LAYER: usize = 10;

// ============================================================================
// COLLISION & EFFECT DATA
// ============================================================================

/// Data describing a single collision event.
#[derive(Clone, Debug)]
struct CollisionInfo {
    point: Vec2,
    /// Magnitude of the component of relative velocity along the collision normal.
    impact_velocity: f32,
    /// Surface normal pointing from the obstacle toward the ball.
    normal: Vec2,
    material1: Material,
    material2: Material,
}

/// Visual ripple effect drawn around collisions.
#[derive(Clone, Debug)]
struct Ripple {
    origin: Vec2,
    radius: f32,
    max_radius: f32,
    opacity: f32,
    speed: f32,
    /// Normalized impact strength; generally in [0.2, 2.0].
    intensity: f32,
    color: Color,
}

impl Ripple {
    fn new(
        origin: Vec2,
        impact_velocity: f32,
        mat1: Material,
        mat2: Material,
        normal: Vec2,
    ) -> Self {
        let normalized_impact = (impact_velocity / 500.0).clamp(0.2, 2.0);

        let color = Color::new(
            (mat1.color_base[0] + mat2.color_base[0]) * 0.5,
            (mat1.color_base[1] + mat2.color_base[1]) * 0.5,
            (mat1.color_base[2] + mat2.color_base[2]) * 0.5,
            1.0,
        );

        // Offset slightly along the collision normal to avoid z-fighting.
        let offset_origin = origin + normal * 2.0;

        let max_radius = MIN_RIPPLE_RADIUS
            + (MAX_RIPPLE_RADIUS - MIN_RIPPLE_RADIUS) * normalized_impact.min(1.0);
        let base_opacity = 0.4 + 0.4 * normalized_impact.min(1.0);
        let speed = RIPPLE_BASE_SPEED * (0.8 + 0.4 * normalized_impact);

        Self {
            origin: offset_origin,
            radius: 0.0,
            max_radius,
            opacity: base_opacity,
            speed,
            intensity: normalized_impact,
            color,
        }
    }

    fn update(&mut self, dt: f32) {
        self.radius += self.speed * dt;

        let t = (self.radius / self.max_radius).clamp(0.0, 1.0);
        let fade_factor = 1.0 - t;
        self.opacity = fade_factor * (0.4 + 0.4 * self.intensity.min(1.0));
    }

    fn is_alive(&self) -> bool {
        self.radius < self.max_radius && self.opacity > 0.01
    }
}

/// Expanding ring used to visualize a "sound wave" after strong impacts.
#[derive(Clone, Debug)]
struct SoundWave {
    origin: Vec2,
    radius: f32,
    intensity: f32,
    max_radius: f32,
}

impl SoundWave {
    fn new(origin: Vec2, intensity: f32) -> Self {
        Self {
            origin,
            radius: 0.0,
            intensity,
            max_radius: 400.0,
        }
    }

    fn update(&mut self, dt: f32) {
        self.radius += SOUND_WAVE_SPEED * dt;
    }

    fn is_alive(&self) -> bool {
        self.radius < self.max_radius
    }

    fn opacity(&self) -> f32 {
        let t = (self.radius / self.max_radius).clamp(0.0, 1.0);
        (1.0 - t) * self.intensity * 0.3
    }
}

// ============================================================================
// BALL ENTITY
// ============================================================================

/// A single ball with position, motion, and visual state.
#[derive(Clone, Debug)]
struct Ball {
    pos: Vec2,
    vel: Vec2,
    radius: f32,
    mass: f32,

    angle: f32,
    angular_velocity: f32,

    material: Material,
    base_color: Color,
    /// Temperature factor used to tint color (1.0 = baseline).
    temperature: f32,

    /// Historic positions for rendering motion trails.
    trail: VecDeque<Vec2>,
}

impl Ball {
    fn new(spawn_pos: Vec2) -> Self {
        let radius = rand::gen_range(MIN_BALL_RADIUS, MAX_BALL_RADIUS);

        let material = match rand::gen_range(0, 3) {
            0 => MATERIAL_RUBBER,
            1 => MATERIAL_STEEL,
            _ => MATERIAL_GLASS,
        };

        // Approximate mass as density * 2D area (πr²), scaled down.
        let mass = material.density * PI * radius * radius / 100.0;

        // Slight color variation for visual interest.
        let color_variation = rand::gen_range(0.9, 1.1);
        let base_color = Color::new(
            (material.color_base[0] * color_variation).min(1.0),
            (material.color_base[1] * color_variation).min(1.0),
            (material.color_base[2] * color_variation).min(1.0),
            1.0,
        );

        let vel = Vec2::new(
            rand::gen_range(-400.0, 400.0),
            rand::gen_range(-300.0, -100.0),
        );
        let angular_velocity = rand::gen_range(-5.0, 5.0);

        Self {
            pos: spawn_pos,
            vel,
            radius,
            mass,
            angle: 0.0,
            angular_velocity,
            material,
            base_color,
            temperature: 1.0,
            trail: VecDeque::with_capacity(TRAIL_LENGTH_MAX),
        }
    }

    /// Advance the ball physics by `dt`.
    ///
    /// Wall collisions are reported by pushing `CollisionInfo` into `out_collisions`.
    fn update(
        &mut self,
        dt: f32,
        layers: &[PolygonLayer],
        out_collisions: &mut Vec<CollisionInfo>,
    ) {
        self.apply_forces(dt);
        self.angle += self.angular_velocity * dt;

        // Substep integration for robust wall collision handling.
        let sub_dt = dt / SUBSTEPS as f32;
        for _ in 0..SUBSTEPS {
            let prev_pos = self.pos;
            self.pos += self.vel * sub_dt;

            if let Some(info) = self.handle_wall_collisions(layers, prev_pos) {
                out_collisions.push(info);
                // Stop on first collision to avoid over-resolving.
                break;
            }
        }

        self.update_trail();
    }

    fn apply_forces(&mut self, dt: f32) {
        // Gravity: v += a dt.
        self.vel.y += GRAVITY * dt;

        // Quadratic drag (very simplified).
        let speed_sq = self.vel.length_squared();
        if speed_sq > 0.01 {
            let speed = speed_sq.sqrt();
            let drag_force =
                0.5 * AIR_DENSITY * speed_sq * self.material.drag_coefficient * self.radius;
            let drag_accel = drag_force / self.mass;
            self.vel += -self.vel / speed * drag_accel * dt;
        }

        // Rotational damping (dt-correct).
        self.angular_velocity *= (-ANGULAR_DAMPING_PER_SEC * dt).exp();

        // Temperature relaxes back toward 1.0 (toy model).
        let cooling_rate = (1.0 - TEMPERATURE_DECAY) * dt * 10.0;
        self.temperature += (1.0 - self.temperature) * cooling_rate;
    }

    fn handle_wall_collisions(
        &mut self,
        layers: &[PolygonLayer],
        previous_pos: Vec2,
    ) -> Option<CollisionInfo> {
        for layer in layers {
            let Some((contact_point, normal)) = layer.check_collision(self.pos, self.radius) else {
                continue;
            };

            // How hard did we hit along the surface normal?
            let impact_velocity = (-self.vel.dot(normal)).max(0.0);

            // Rewind to avoid tunneling.
            self.pos = previous_pos;

            // Combine restitution (simple average).
            let combined_restitution =
                (self.material.restitution + WALL_MATERIAL.restitution) * 0.5;

            // Correct restitution reflection: v' = v - (1 + e) (v·n) n
            self.vel = reflect_velocity(self.vel, normal, combined_restitution);

            // Apply friction to angular velocity (toy model).
            let tangent = Vec2::new(-normal.y, normal.x);
            let slip_speed = self.vel.dot(tangent) - self.angular_velocity * self.radius;
            let friction_impulse = slip_speed * self.material.friction * 0.1;
            self.angular_velocity += friction_impulse / self.radius;

            // Heat from impact.
            self.temperature = (self.temperature + impact_velocity * 0.001).min(3.0);

            // Enforce a minimum speed to avoid balls freezing on surfaces.
            let speed = self.vel.length();
            if speed < MIN_VELOCITY {
                self.vel = if speed > 0.01 {
                    self.vel / speed * MIN_VELOCITY
                } else {
                    // If the velocity is almost zero, push away from the wall.
                    normal * MIN_VELOCITY
                };
            }

            // Separate ball from the wall.
            self.pos = contact_point + normal * (self.radius + 0.5);

            return Some(CollisionInfo {
                point: contact_point,
                impact_velocity,
                normal,
                material1: self.material,
                material2: WALL_MATERIAL,
            });
        }

        None
    }

    fn update_trail(&mut self) {
        if self.trail.len() == TRAIL_LENGTH_MAX {
            self.trail.pop_front();
        }
        self.trail.push_back(self.pos);
    }

    /// Resolve a collision with another ball using an impulse model.
    fn resolve_ball_collision(&mut self, other: &mut Ball, normal: Vec2, restitution: f32) {
        let relative_velocity = self.vel - other.vel;
        let vel_along_normal = relative_velocity.dot(normal);

        // If balls are moving apart along the normal, nothing to do.
        if vel_along_normal >= 0.0 {
            return;
        }

        let inv_mass_sum = (1.0 / self.mass) + (1.0 / other.mass);
        if inv_mass_sum <= f32::EPSILON {
            return;
        }

        // Impulse magnitude:
        // j = -(1 + e) (v_rel · n) / (1/m1 + 1/m2)
        let e = restitution.clamp(0.0, 1.0);
        let j = -(1.0 + e) * vel_along_normal / inv_mass_sum;
        let impulse = j * normal;

        self.vel += impulse / self.mass;
        other.vel -= impulse / other.mass;

        // Transfer some tangential motion into spin (toy model).
        let tangent = Vec2::new(-normal.y, normal.x);
        let self_tangent = self.vel.dot(tangent);
        let other_tangent = other.vel.dot(tangent);

        self.angular_velocity -= self_tangent * self.material.friction * 0.05 / self.radius;
        other.angular_velocity += other_tangent * other.material.friction * 0.05 / other.radius;

        // Heat from impact (visual only).
        let impact_strength = j.abs();
        let heat = impact_strength * 0.001;
        self.temperature += heat;
        other.temperature += heat;

        // Positional correction to reduce overlap.
        let delta = self.pos - other.pos;
        let distance = delta.length();
        let min_distance = self.radius + other.radius;

        if distance > f32::EPSILON && distance < min_distance {
            let overlap = min_distance - distance;
            let correction = normal * (overlap * 0.5 + 0.1);
            self.pos += correction;
            other.pos -= correction;
        }
    }

    fn draw(&self) {
        self.draw_trail();
        self.draw_body();
        self.draw_spin_indicator();
        self.draw_highlight();
        self.draw_material_indicator();
    }

    fn draw_trail(&self) {
        if self.trail.is_empty() {
            return;
        }

        let len = self.trail.len() as f32;
        for (i, &pos) in self.trail.iter().enumerate() {
            let alpha = (i as f32 / len) * 0.3;
            let mut color = self.base_color;
            color.a = alpha;
            draw_circle(pos.x, pos.y, self.radius * 0.5, color);
        }
    }

    fn current_color(&self) -> Color {
        Color::new(
            (self.base_color.r * self.temperature).min(1.0),
            self.base_color.g / self.temperature.sqrt(),
            self.base_color.b / self.temperature,
            self.base_color.a,
        )
    }

    fn draw_body(&self) {
        draw_circle(self.pos.x, self.pos.y, self.radius, self.current_color());
    }

    fn draw_spin_indicator(&self) {
        if self.angular_velocity.abs() <= 0.1 {
            return;
        }

        let color = Color::new(1.0, 1.0, 1.0, 0.5);
        let (sin, cos) = self.angle.sin_cos();
        let end = self.pos + Vec2::new(cos, sin) * (self.radius * SPIN_INDICATOR_LENGTH);

        draw_line(self.pos.x, self.pos.y, end.x, end.y, 1.5, color);
    }

    fn draw_highlight(&self) {
        let color = self.current_color();
        let highlight = Color::new(
            (color.r + 0.3).min(1.0),
            (color.g + 0.3).min(1.0),
            (color.b + 0.3).min(1.0),
            0.6,
        );

        draw_circle(
            self.pos.x - self.radius * 0.3,
            self.pos.y - self.radius * 0.3,
            self.radius * 0.5,
            highlight,
        );
    }

    fn draw_material_indicator(&self) {
        let indicator_color = match self.material.kind {
            MaterialKind::Rubber => GREEN,
            MaterialKind::Glass => YELLOW,
            MaterialKind::Steel => RED,
        };

        draw_circle(self.pos.x, self.pos.y, 2.0, indicator_color);
    }
}

// ============================================================================
// POLYGON LAYER (ARENA)
// ============================================================================

/// A single polygonal layer in the arena (e.g. a rotating decagon).
#[derive(Debug)]
struct PolygonLayer {
    center: Vec2,
    base_vertices: Vec<Vec2>,
    world_vertices: Vec<Vec2>,
    active_edges: Vec<bool>,
    rotation: f32,
    rotation_speed: f32,
    layer_index: usize,
    color: Color,
}

impl PolygonLayer {
    fn new(center: Vec2, radius: f32, layer_index: usize, total_layers: usize) -> Self {
        let mut base_vertices = Vec::with_capacity(SIDES_PER_LAYER);
        for i in 0..SIDES_PER_LAYER {
            let angle = (i as f32) * TAU / SIDES_PER_LAYER as f32 - PI / SIDES_PER_LAYER as f32;
            let (sin, cos) = angle.sin_cos();
            base_vertices.push(Vec2::new(cos * radius, sin * radius));
        }

        // Start with all edges active, then remove some for inner layers.
        let mut active_edges = vec![true; SIDES_PER_LAYER];
        if layer_index > 0 {
            let edges_to_remove = rand::gen_range(2, 4);
            let mut removed = 0;
            while removed < edges_to_remove {
                let idx = rand::gen_range(0, SIDES_PER_LAYER);
                if active_edges[idx] {
                    active_edges[idx] = false;
                    removed += 1;
                }
            }
        }

        let clockwise = rand::gen_range(0, 2) == 0;
        let speed_variation = rand::gen_range(0.5, 1.5);
        let rotation_speed =
            ROTATION_SPEED_BASE * speed_variation * if clockwise { 1.0 } else { -1.0 };

        // Inner layers are darker.
        let brightness = 0.8 - (layer_index as f32 / total_layers as f32) * 0.4;
        let color = Color::new(brightness, brightness, (brightness * 1.1).min(1.0), 1.0);

        let rotation = rand::gen_range(0.0, TAU);

        let mut layer = Self {
            center,
            base_vertices,
            world_vertices: Vec::with_capacity(SIDES_PER_LAYER),
            active_edges,
            rotation,
            rotation_speed,
            layer_index,
            color,
        };
        layer.rebuild_world_vertices();
        layer
    }

    fn update(&mut self, dt: f32) {
        self.rotation = (self.rotation + self.rotation_speed * dt) % TAU;
        self.rebuild_world_vertices();
    }

    fn rebuild_world_vertices(&mut self) {
        self.world_vertices.clear();
        let (sin, cos) = self.rotation.sin_cos();
        self.world_vertices.extend(
            self.base_vertices
                .iter()
                .map(|&local| rotate_point_sincos(local, sin, cos) + self.center),
        );
    }

    fn vertices(&self) -> &[Vec2] {
        &self.world_vertices
    }

    fn draw(&self) {
        let vertices = self.vertices();
        let n = vertices.len();

        // Fill the outermost layer to give a “floor” effect.
        if self.layer_index == 0 {
            for i in 0..n {
                if self.active_edges[i] {
                    let v1 = vertices[i];
                    let v2 = vertices[(i + 1) % n];
                    draw_triangle(self.center, v1, v2, Color::new(0.1, 0.15, 0.2, 0.3));
                }
            }
        }

        // Draw active edges.
        for i in 0..n {
            if self.active_edges[i] {
                let v1 = vertices[i];
                let v2 = vertices[(i + 1) % n];
                let thickness = (3.0 - self.layer_index as f32 * 0.5).max(1.0);
                draw_line(v1.x, v1.y, v2.x, v2.y, thickness, self.color);
            }
        }

        // Draw vertex markers where at least one adjacent edge is active.
        for (i, &v) in vertices.iter().enumerate() {
            let prev_edge = if i == 0 { n - 1 } else { i - 1 };
            if self.active_edges[i] || self.active_edges[prev_edge] {
                let size = (2.5 - self.layer_index as f32 * 0.3).max(1.0);
                draw_circle(v.x, v.y, size, YELLOW);
            }
        }
    }

    fn draw_ripple(&self, ripple: &Ripple) {
        let rings = (2.0 + ripple.intensity * 2.0) as i32;
        if rings <= 0 {
            return;
        }

        let vertices = self.vertices();

        for i in 0..rings {
            let offset = i as f32 * (8.0 + 4.0 * (1.0 - ripple.intensity));
            let radius = ripple.radius - offset;
            if radius <= 0.0 || radius > ripple.max_radius {
                continue;
            }

            let ring_opacity = ripple.opacity * (1.0 - i as f32 / rings as f32);

            for seg in 0..RING_SEGMENTS {
                let angle1 = (seg as f32) * TAU / RING_SEGMENTS as f32;
                let angle2 = ((seg + 1) as f32) * TAU / RING_SEGMENTS as f32;

                let (sin1, cos1) = angle1.sin_cos();
                let (sin2, cos2) = angle2.sin_cos();

                let p1 = ripple.origin + Vec2::new(cos1 * radius, sin1 * radius);
                let p2 = ripple.origin + Vec2::new(cos2 * radius, sin2 * radius);

                if point_inside_polygon(vertices, &self.active_edges, p1)
                    && point_inside_polygon(vertices, &self.active_edges, p2)
                {
                    let thickness = ((1.5 + ripple.intensity) - i as f32 * 0.5).max(0.5);
                    let mut color = ripple.color;
                    color.a = ring_opacity;
                    draw_line(p1.x, p1.y, p2.x, p2.y, thickness, color);
                }
            }
        }
    }

    fn draw_sound_wave(&self, wave: &SoundWave) {
        let opacity = wave.opacity();
        if opacity <= 0.01 {
            return;
        }

        let vertices = self.vertices();
        for seg in 0..RING_SEGMENTS {
            let angle1 = (seg as f32) * TAU / RING_SEGMENTS as f32;
            let angle2 = ((seg + 1) as f32) * TAU / RING_SEGMENTS as f32;

            let (sin1, cos1) = angle1.sin_cos();
            let (sin2, cos2) = angle2.sin_cos();

            let p1 = wave.origin + Vec2::new(cos1 * wave.radius, sin1 * wave.radius);
            let p2 = wave.origin + Vec2::new(cos2 * wave.radius, sin2 * wave.radius);

            if point_inside_polygon(vertices, &self.active_edges, p1)
                && point_inside_polygon(vertices, &self.active_edges, p2)
            {
                draw_line(
                    p1.x,
                    p1.y,
                    p2.x,
                    p2.y,
                    1.0,
                    Color::new(1.0, 1.0, 1.0, opacity),
                );
            }
        }
    }

    /// Check for collision between this polygon's edges and a circle.
    ///
    /// Returns the closest collision point and normal if any.
    fn check_collision(&self, circle_center: Vec2, radius: f32) -> Option<(Vec2, Vec2)> {
        let vertices = self.vertices();
        let n = vertices.len();

        // Early-out: if center isn't inside polygon, we skip.
        if !point_inside_polygon(vertices, &self.active_edges, circle_center) {
            return None;
        }

        let mut closest: Option<(Vec2, Vec2, f32)> = None;

        for i in 0..n {
            if !self.active_edges[i] {
                continue;
            }

            let v1 = vertices[i];
            let v2 = vertices[(i + 1) % n];

            if let Some((point, normal, distance)) =
                circle_line_collision(circle_center, radius, v1, v2)
            {
                match closest {
                    None => closest = Some((point, normal, distance)),
                    Some((_, _, best_dist)) if distance < best_dist => {
                        closest = Some((point, normal, distance))
                    }
                    _ => {}
                }
            }
        }

        closest.map(|(p, n, _)| (p, n))
    }
}

// ============================================================================
// GEOMETRY & PHYSICS HELPERS
// ============================================================================

#[inline]
fn rotate_point_sincos(point: Vec2, sin: f32, cos: f32) -> Vec2 {
    Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

/// Compute intersection between a circle and a line segment.
fn circle_line_collision(
    circle_center: Vec2,
    radius: f32,
    line_start: Vec2,
    line_end: Vec2,
) -> Option<(Vec2, Vec2, f32)> {
    let line_vec = line_end - line_start;
    let center_to_start = circle_center - line_start;

    let line_length_sq = line_vec.length_squared();
    if line_length_sq <= f32::EPSILON {
        return None;
    }

    // Project center onto line segment.
    let t = (center_to_start.dot(line_vec) / line_length_sq).clamp(0.0, 1.0);
    let closest_point = line_start + line_vec * t;
    let delta = circle_center - closest_point;
    let distance = delta.length();

    if distance > radius {
        return None;
    }

    let normal = if distance > 0.001 {
        delta / distance
    } else {
        Vec2::new(-line_vec.y, line_vec.x).normalize_or_zero()
    };

    Some((closest_point, normal, distance))
}

/// Reflect a velocity vector across a surface normal.
///
/// The normal is assumed to be a unit vector pointing from the surface toward the object.
///
/// With restitution `e`:
/// - `e = 0` cancels the normal component (no bounce)
/// - `e = 1` reflects fully (perfectly elastic)
fn reflect_velocity(v: Vec2, n: Vec2, restitution: f32) -> Vec2 {
    let e = restitution.clamp(0.0, 1.0);
    let vn = v.dot(n);
    if vn >= 0.0 {
        return v;
    }

    // v' = v - (1 + e) (v·n) n
    v - (1.0 + e) * vn * n
}

/// Ray-cast point-in-polygon test.
///
/// `active_edges` controls which edges form the “solid” boundary.
fn point_inside_polygon(vertices: &[Vec2], active_edges: &[bool], point: Vec2) -> bool {
    let n = vertices.len();
    let mut inside = false;

    for i in 0..n {
        if !active_edges[i] {
            continue;
        }

        let v1 = vertices[i];
        let v2 = vertices[(i + 1) % n];

        let intersect = ((v1.y > point.y) != (v2.y > point.y))
            && (point.x < (v2.x - v1.x) * (point.y - v1.y) / (v2.y - v1.y + f32::EPSILON) + v1.x);

        if intersect {
            inside = !inside;
        }
    }

    inside
}

// ============================================================================
// SIMULATION ROOT
// ============================================================================

struct Simulation {
    balls: Vec<Ball>,
    layers: Vec<PolygonLayer>,
    ripples: Vec<Ripple>,
    sound_waves: Vec<SoundWave>,

    // Scratch buffer reused each frame.
    frame_collisions: Vec<CollisionInfo>,

    show_info: bool,
    total_collisions: usize,
    total_energy: f32,
}

impl Simulation {
    fn new() -> Self {
        let center = Vec2::new(screen_width() / 2.0, screen_height() / 2.0);
        let balls = Self::spawn_balls(center);
        let layers = Self::create_layers(center);

        Self {
            balls,
            layers,
            ripples: Vec::with_capacity(128),
            sound_waves: Vec::with_capacity(64),
            frame_collisions: Vec::with_capacity(64),
            show_info: true,
            total_collisions: 0,
            total_energy: 0.0,
        }
    }

    fn spawn_balls(center: Vec2) -> Vec<Ball> {
        let ball_count = rand::gen_range(MIN_BALLS, MAX_BALLS + 1);
        let mut balls = Vec::with_capacity(ball_count);

        for i in 0..ball_count {
            let angle = (i as f32) * TAU / ball_count as f32;
            let (sin, cos) = angle.sin_cos();

            let spawn_radius = 100.0 + rand::gen_range(-20.0, 20.0);
            let x = center.x + cos * spawn_radius;
            let y = center.y + sin * spawn_radius - 50.0;
            balls.push(Ball::new(Vec2::new(x, y)));
        }

        balls
    }

    fn create_layers(center: Vec2) -> Vec<PolygonLayer> {
        let layer_count = rand::gen_range(MIN_LAYERS, MAX_LAYERS + 1);
        let mut layers = Vec::with_capacity(layer_count);

        for i in 0..layer_count {
            let radius = 280.0 - i as f32 * LAYER_SPACING;
            layers.push(PolygonLayer::new(center, radius, i, layer_count));
        }

        layers
    }

    fn handle_input(&mut self) {
        if is_key_pressed(KeyCode::Space) {
            self.show_info = !self.show_info;
        }

        if is_key_pressed(KeyCode::R) {
            *self = Simulation::new();
        }
    }

    fn update(&mut self, dt: f32) {
        for layer in &mut self.layers {
            layer.update(dt);
        }

        self.frame_collisions.clear();

        for ball in &mut self.balls {
            ball.update(dt, &self.layers, &mut self.frame_collisions);
        }

        self.resolve_ball_collisions();
        self.spawn_effects_from_collisions();
        self.update_effects(dt);
        self.update_energy();
    }

    fn resolve_ball_collisions(&mut self) {
        let len = self.balls.len();

        for i in 0..len {
            for j in (i + 1)..len {
                let pos_i = self.balls[i].pos;
                let pos_j = self.balls[j].pos;
                let delta = pos_i - pos_j;
                let distance = delta.length();
                let min_distance = self.balls[i].radius + self.balls[j].radius;

                if distance <= 0.01 || distance >= min_distance {
                    continue;
                }

                let normal = delta / distance;
                let relative_velocity = self.balls[i].vel - self.balls[j].vel;
                let impact_speed = relative_velocity.dot(normal);

                if impact_speed >= 0.0 {
                    continue;
                }

                // Record collision for effects.
                let contact_point = self.balls[i].pos - normal * self.balls[i].radius;
                self.frame_collisions.push(CollisionInfo {
                    point: contact_point,
                    impact_velocity: impact_speed.abs(),
                    normal,
                    material1: self.balls[i].material,
                    material2: self.balls[j].material,
                });

                // Split mutable borrow to satisfy the borrow checker.
                let (left, right) = self.balls.split_at_mut(j);
                let ball1 = &mut left[i];
                let ball2 = &mut right[0];

                let combined_restitution =
                    (ball1.material.restitution + ball2.material.restitution) * 0.5;
                ball1.resolve_ball_collision(ball2, normal, combined_restitution);
            }
        }
    }

    fn spawn_effects_from_collisions(&mut self) {
        for collision in &self.frame_collisions {
            self.ripples.push(Ripple::new(
                collision.point,
                collision.impact_velocity,
                collision.material1,
                collision.material2,
                collision.normal,
            ));

            if collision.impact_velocity > 200.0 {
                self.sound_waves.push(SoundWave::new(
                    collision.point,
                    collision.impact_velocity / 1000.0,
                ));
            }

            self.total_collisions += 1;
        }
    }

    fn update_effects(&mut self, dt: f32) {
        for ripple in &mut self.ripples {
            ripple.update(dt);
        }
        self.ripples.retain(|r| r.is_alive());

        for wave in &mut self.sound_waves {
            wave.update(dt);
        }
        self.sound_waves.retain(|w| w.is_alive());
    }

    fn update_energy(&mut self) {
        self.total_energy = self
            .balls
            .iter()
            .map(|b| {
                let translational = 0.5 * b.mass * b.vel.length_squared();
                let rotational =
                    0.5 * b.mass * b.radius * b.radius * b.angular_velocity * b.angular_velocity;
                translational + rotational
            })
            .sum();
    }

    fn draw(&self) {
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));

        // Effects are clipped against the outermost layer.
        if let Some(outer_layer) = self.layers.first() {
            for ripple in &self.ripples {
                outer_layer.draw_ripple(ripple);
            }
            for wave in &self.sound_waves {
                outer_layer.draw_sound_wave(wave);
            }
        }

        // Draw layers from outer to inner so the outer fill doesn't cover inner geometry.
        for layer in &self.layers {
            layer.draw();
        }

        for ball in &self.balls {
            ball.draw();
        }

        if self.show_info {
            self.draw_info_overlay();
        }
    }

    fn draw_info_overlay(&self) {
        let info_color = GREEN;

        draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 20.0, info_color);

        let mut rubber = 0;
        let mut glass = 0;
        let mut steel = 0;
        for b in &self.balls {
            match b.material.kind {
                MaterialKind::Rubber => rubber += 1,
                MaterialKind::Glass => glass += 1,
                MaterialKind::Steel => steel += 1,
            }
        }

        draw_text(
            &format!(
                "Balls: {} (Rubber={} Glass={} Steel={})",
                self.balls.len(),
                rubber,
                glass,
                steel
            ),
            10.0,
            40.0,
            20.0,
            info_color,
        );

        draw_text(
            &format!("Total Collisions: {}", self.total_collisions),
            10.0,
            60.0,
            20.0,
            info_color,
        );

        draw_text(
            &format!("System Energy: {:.0} J (approx.)", self.total_energy),
            10.0,
            80.0,
            20.0,
            info_color,
        );

        draw_text(
            "Effects: Ripples + Sound Waves",
            10.0,
            100.0,
            20.0,
            info_color,
        );

        // --- Material legend ----------------------------------------------
        draw_text("Materials:", 10.0, 130.0, 16.0, GRAY);

        draw_circle(20.0, 150.0, 5.0, Color::new(0.8, 0.3, 0.3, 1.0));
        draw_text(
            "Rubber: Light, Bouncy, High Friction",
            35.0,
            155.0,
            14.0,
            GRAY,
        );

        draw_circle(20.0, 170.0, 5.0, Color::new(0.7, 0.7, 0.8, 1.0));
        draw_text("Steel: Heavy, Less Bouncy, Smooth", 35.0, 175.0, 14.0, GRAY);

        draw_circle(20.0, 190.0, 5.0, Color::new(0.6, 0.8, 0.9, 1.0));
        draw_text(
            "Glass: Medium, Very Bouncy, Slippery",
            35.0,
            195.0,
            14.0,
            GRAY,
        );

        draw_text(
            "Controls: SPACE = Toggle Info | R = New Simulation",
            10.0,
            220.0,
            20.0,
            GRAY,
        );
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

#[macroquad::main("Advanced Physics Simulation (Idiomatic Rust 2024)")]
async fn main() {
    rand::srand(macroquad::miniquad::date::now() as u64);

    let mut sim = Simulation::new();

    let mut accumulator = 0.0_f32;

    loop {
        let frame_dt = get_frame_time().min(MAX_FRAME_TIME);
        accumulator = (accumulator + frame_dt).min(MAX_ACCUMULATED_TIME);

        sim.handle_input();

        let mut steps = 0;
        while accumulator >= FIXED_DT && steps < MAX_STEPS_PER_FRAME {
            sim.update(FIXED_DT);
            accumulator -= FIXED_DT;
            steps += 1;
        }

        sim.draw();
        next_frame().await;
    }
}
