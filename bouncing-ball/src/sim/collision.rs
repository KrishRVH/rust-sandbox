use macroquad::prelude::*;

use crate::config::PhysicsConfig;
use crate::geometry::{
    angular_velocity_at, circle_segment_contact, closest_point_on_segment, point_inside_polygon,
};

use super::arena::{Arena, ArenaLayer};
use super::ball::Ball;
use super::material::{MaterialKind, WALL_MATERIAL, combine_friction, combine_restitution};

const MAX_RECORDED_COLLISION_EVENTS: usize = 128;

#[derive(Clone, Copy, Debug)]
pub enum CollisionKind {
    BallBall,
    BallWall,
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionEvent {
    pub point: Vec2,
    pub normal: Vec2,
    pub impact_speed: f32,
    pub material_a: MaterialKind,
    pub material_b: MaterialKind,
    pub kind: CollisionKind,
}

pub fn solve_wall_contacts(
    balls: &mut [Ball],
    arena: &Arena,
    physics: &PhysicsConfig,
    events: &mut Vec<CollisionEvent>,
) {
    for ball in balls {
        for layer in &arena.layers {
            solve_ball_layer_contact(ball, layer, arena.center, physics, events);
        }

        if let Some(outer) = arena.outer() {
            enforce_outer_containment(ball, outer, arena.center, physics, events);
        }
    }
}

pub fn solve_ball_contacts(
    balls: &mut [Ball],
    physics: &PhysicsConfig,
    events: &mut Vec<CollisionEvent>,
) {
    for i in 0..balls.len() {
        for j in (i + 1)..balls.len() {
            let delta = balls[i].position - balls[j].position;
            let distance_sq = delta.length_squared();
            let min_distance = balls[i].radius + balls[j].radius;

            if distance_sq >= min_distance * min_distance {
                continue;
            }

            let distance = distance_sq.sqrt();
            let normal = if distance > 0.0001 {
                delta / distance
            } else {
                let fallback = balls[i].previous_position - balls[j].previous_position;
                fallback.normalize_or_zero()
            };

            if normal.length_squared() <= f32::EPSILON {
                continue;
            }

            let (left, right) = balls.split_at_mut(j);
            let ball_a = &mut left[i];
            let ball_b = &mut right[0];

            resolve_ball_pair(ball_a, ball_b, normal, distance, physics, events);
        }
    }
}

fn solve_ball_layer_contact(
    ball: &mut Ball,
    layer: &ArenaLayer,
    arena_center: Vec2,
    physics: &PhysicsConfig,
    events: &mut Vec<CollisionEvent>,
) {
    for edge_index in 0..layer.world_vertices.len() {
        let Some((start, end)) = layer.active_segment(edge_index) else {
            continue;
        };

        let Some(contact) = circle_segment_contact(ball.position, ball.radius, start, end) else {
            continue;
        };

        let mut normal = if layer.index == 0 {
            (arena_center - contact.point).normalize_or_zero()
        } else {
            contact.normal
        };

        if normal.length_squared() <= f32::EPSILON {
            let edge = end - start;
            normal = Vec2::new(-edge.y, edge.x).normalize_or_zero();
        }

        let wall_velocity = angular_velocity_at(contact.point, layer.center, layer.rotation_speed);
        resolve_ball_wall(
            ball,
            contact.point,
            contact.distance,
            normal,
            wall_velocity,
            physics,
            events,
        );
    }
}

fn enforce_outer_containment(
    ball: &mut Ball,
    outer: &ArenaLayer,
    arena_center: Vec2,
    physics: &PhysicsConfig,
    events: &mut Vec<CollisionEvent>,
) {
    if point_inside_polygon(&outer.world_vertices, &outer.active_edges, ball.position) {
        return;
    }

    let Some((point, distance)) = closest_active_point(ball.position, outer) else {
        return;
    };

    let normal = (arena_center - point).normalize_or_zero();
    if normal.length_squared() <= f32::EPSILON {
        return;
    }

    let wall_velocity = angular_velocity_at(point, outer.center, outer.rotation_speed);
    resolve_ball_wall(
        ball,
        point,
        -distance,
        normal,
        wall_velocity,
        physics,
        events,
    );
    ball.position = point + normal * (ball.radius + physics.penetration_slop);
}

fn closest_active_point(point: Vec2, layer: &ArenaLayer) -> Option<(Vec2, f32)> {
    let mut best: Option<(Vec2, f32)> = None;
    for i in 0..layer.world_vertices.len() {
        let Some((start, end)) = layer.active_segment(i) else {
            continue;
        };
        let candidate = closest_point_on_segment(point, start, end);
        let distance = (point - candidate).length();
        match best {
            None => best = Some((candidate, distance)),
            Some((_, best_distance)) if distance < best_distance => {
                best = Some((candidate, distance));
            }
            _ => {}
        }
    }
    best
}

fn resolve_ball_wall(
    ball: &mut Ball,
    point: Vec2,
    distance: f32,
    normal: Vec2,
    wall_velocity: Vec2,
    physics: &PhysicsConfig,
    events: &mut Vec<CollisionEvent>,
) {
    let penetration = ball.radius - distance;
    if penetration > 0.0 {
        ball.position += normal * (penetration + physics.penetration_slop);
    }

    let material = ball.material();
    let relative_velocity = ball.velocity - wall_velocity;
    let normal_speed = relative_velocity.dot(normal);

    if normal_speed >= -physics.restitution_slop {
        return;
    }

    let restitution = combine_restitution(material, WALL_MATERIAL);
    let normal_impulse = -(1.0 + restitution) * normal_speed / ball.inverse_mass.max(f32::EPSILON);
    ball.velocity += normal * normal_impulse * ball.inverse_mass;

    apply_wall_friction(
        ball,
        normal,
        relative_velocity,
        normal_impulse,
        material.friction,
        physics,
    );
    ball.add_heat_from_impulse(normal_impulse);

    record_event(
        events,
        CollisionEvent {
            point,
            normal,
            impact_speed: -normal_speed,
            material_a: material.kind,
            material_b: WALL_MATERIAL.kind,
            kind: CollisionKind::BallWall,
        },
    );
}

fn resolve_ball_pair(
    ball_a: &mut Ball,
    ball_b: &mut Ball,
    normal: Vec2,
    distance: f32,
    physics: &PhysicsConfig,
    events: &mut Vec<CollisionEvent>,
) {
    let material_a = ball_a.material();
    let material_b = ball_b.material();
    let inverse_mass_sum = ball_a.inverse_mass + ball_b.inverse_mass;
    if inverse_mass_sum <= f32::EPSILON {
        return;
    }

    let min_distance = ball_a.radius + ball_b.radius;
    let penetration = min_distance - distance;
    if penetration > 0.0 {
        let correction = ((penetration - physics.penetration_slop).max(0.0) / inverse_mass_sum)
            * physics.position_correction_percent;
        ball_a.position += normal * correction * ball_a.inverse_mass;
        ball_b.position -= normal * correction * ball_b.inverse_mass;
    }

    let relative_velocity = ball_a.velocity - ball_b.velocity;
    let normal_speed = relative_velocity.dot(normal);
    if normal_speed >= -physics.restitution_slop {
        return;
    }

    let restitution = combine_restitution(material_a, material_b);
    let normal_impulse = -(1.0 + restitution) * normal_speed / inverse_mass_sum;
    let impulse = normal * normal_impulse;
    ball_a.velocity += impulse * ball_a.inverse_mass;
    ball_b.velocity -= impulse * ball_b.inverse_mass;

    apply_pair_friction(ball_a, ball_b, normal, relative_velocity, normal_impulse);

    ball_a.add_heat_from_impulse(normal_impulse);
    ball_b.add_heat_from_impulse(normal_impulse);

    record_event(
        events,
        CollisionEvent {
            point: ball_a.position - normal * ball_a.radius,
            normal,
            impact_speed: -normal_speed,
            material_a: material_a.kind,
            material_b: material_b.kind,
            kind: CollisionKind::BallBall,
        },
    );
}

fn record_event(events: &mut Vec<CollisionEvent>, event: CollisionEvent) {
    if events.len() < MAX_RECORDED_COLLISION_EVENTS {
        events.push(event);
    }
}

fn apply_wall_friction(
    ball: &mut Ball,
    normal: Vec2,
    relative_velocity: Vec2,
    normal_impulse: f32,
    friction: f32,
    _physics: &PhysicsConfig,
) {
    let tangent = Vec2::new(-normal.y, normal.x);
    let tangential_speed = relative_velocity.dot(tangent) - ball.angular_velocity * ball.radius;
    let inverse_inertia = ball.inverse_inertia();
    let tangent_mass = ball.inverse_mass + ball.radius * ball.radius * inverse_inertia;
    if tangent_mass <= f32::EPSILON {
        return;
    }

    let max_friction = friction.max(0.0) * normal_impulse;
    let tangent_impulse = (-tangential_speed / tangent_mass).clamp(-max_friction, max_friction);
    ball.velocity += tangent * tangent_impulse * ball.inverse_mass;
    ball.angular_velocity -= tangent_impulse * ball.radius * inverse_inertia;
}

fn apply_pair_friction(
    ball_a: &mut Ball,
    ball_b: &mut Ball,
    normal: Vec2,
    relative_velocity: Vec2,
    normal_impulse: f32,
) {
    let tangent = Vec2::new(-normal.y, normal.x);
    let tangential_speed = relative_velocity.dot(tangent)
        - ball_a.angular_velocity * ball_a.radius
        - ball_b.angular_velocity * ball_b.radius;

    let inverse_inertia_a = ball_a.inverse_inertia();
    let inverse_inertia_b = ball_b.inverse_inertia();
    let tangent_mass = ball_a.inverse_mass
        + ball_b.inverse_mass
        + ball_a.radius * ball_a.radius * inverse_inertia_a
        + ball_b.radius * ball_b.radius * inverse_inertia_b;

    if tangent_mass <= f32::EPSILON {
        return;
    }

    let friction = combine_friction(ball_a.material(), ball_b.material());
    let max_friction = friction * normal_impulse;
    let tangent_impulse = (-tangential_speed / tangent_mass).clamp(-max_friction, max_friction);
    let impulse = tangent * tangent_impulse;

    ball_a.velocity += impulse * ball_a.inverse_mass;
    ball_b.velocity -= impulse * ball_b.inverse_mass;
    ball_a.angular_velocity -= tangent_impulse * ball_a.radius * inverse_inertia_a;
    ball_b.angular_velocity -= tangent_impulse * ball_b.radius * inverse_inertia_b;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_mass_head_on_collision_swaps_velocity_direction() {
        let physics = PhysicsConfig {
            restitution_slop: 0.0,
            ..PhysicsConfig::default()
        };
        let mut a = Ball::with_params(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            10.0,
            MaterialKind::Glass,
            4,
        );
        let mut b = Ball::with_params(
            Vec2::new(19.0, 0.0),
            Vec2::new(-10.0, 0.0),
            10.0,
            MaterialKind::Glass,
            4,
        );
        a.angular_velocity = 0.0;
        b.angular_velocity = 0.0;
        let mut events = Vec::new();

        resolve_ball_pair(
            &mut a,
            &mut b,
            Vec2::new(-1.0, 0.0),
            19.0,
            &physics,
            &mut events,
        );

        assert!(a.velocity.x < 0.0);
        assert!(b.velocity.x > 0.0);
        assert_eq!(events.len(), 1);
    }
}
