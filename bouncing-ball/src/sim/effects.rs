use macroquad::prelude::*;

use crate::config::EffectConfig;

use super::collision::{CollisionEvent, CollisionKind};
use super::material::Rgb;

#[derive(Clone, Debug)]
pub struct Effects {
    pub ripples: Vec<Ripple>,
    pub shockwaves: Vec<Shockwave>,
    pub normals: Vec<ContactNormal>,
}

impl Effects {
    pub fn new(config: &EffectConfig) -> Self {
        Self {
            ripples: Vec::with_capacity(config.max_ripples),
            shockwaves: Vec::with_capacity(config.max_shockwaves),
            normals: Vec::with_capacity(config.max_normals),
        }
    }

    pub fn spawn_from_events(&mut self, events: &[CollisionEvent], config: &EffectConfig) {
        for event in events {
            push_capped(
                &mut self.ripples,
                config.max_ripples,
                Ripple::new(*event, config),
            );

            if event.impact_speed > 230.0 {
                push_capped(
                    &mut self.shockwaves,
                    config.max_shockwaves,
                    Shockwave::new(event.point, event.impact_speed),
                );
            }

            push_capped(
                &mut self.normals,
                config.max_normals,
                ContactNormal::new(event.point, event.normal, event.impact_speed),
            );
        }
    }

    pub fn update(&mut self, dt: f32, config: &EffectConfig) {
        for ripple in &mut self.ripples {
            ripple.update(dt);
        }
        self.ripples.retain(Ripple::is_alive);

        for shockwave in &mut self.shockwaves {
            shockwave.update(dt, config);
        }
        self.shockwaves.retain(Shockwave::is_alive);

        for normal in &mut self.normals {
            normal.update(dt);
        }
        self.normals.retain(ContactNormal::is_alive);
    }

    pub fn clear(&mut self) {
        self.ripples.clear();
        self.shockwaves.clear();
        self.normals.clear();
    }
}

#[derive(Clone, Debug)]
pub struct Ripple {
    pub origin: Vec2,
    pub radius: f32,
    pub max_radius: f32,
    pub opacity: f32,
    pub speed: f32,
    pub intensity: f32,
    pub color: Rgb,
    pub kind: CollisionKind,
}

impl Ripple {
    pub fn new(event: CollisionEvent, config: &EffectConfig) -> Self {
        let normalized = (event.impact_speed / 520.0).clamp(0.15, 1.8);
        let material_a = event.material_a.material().display_rgb;
        let material_b = event.material_b.material().display_rgb;
        let color = material_a.mix(material_b, 0.5);

        Self {
            origin: event.point + event.normal * 2.0,
            radius: 0.0,
            max_radius: config.ripple_min_radius
                + (config.ripple_max_radius - config.ripple_min_radius) * normalized.min(1.0),
            opacity: 0.38 + 0.28 * normalized.min(1.0),
            speed: config.ripple_base_speed * (0.9 + 0.35 * normalized),
            intensity: normalized,
            color,
            kind: event.kind,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.radius += self.speed * dt;
        let t = (self.radius / self.max_radius).clamp(0.0, 1.0);
        self.opacity *= 1.0 - t * 0.12;
    }

    pub fn is_alive(&self) -> bool {
        self.radius < self.max_radius && self.opacity > 0.01
    }
}

#[derive(Clone, Debug)]
pub struct Shockwave {
    pub origin: Vec2,
    pub radius: f32,
    pub intensity: f32,
    pub max_radius: f32,
}

impl Shockwave {
    pub fn new(origin: Vec2, impact_speed: f32) -> Self {
        let intensity = (impact_speed / 850.0).clamp(0.15, 1.0);
        Self {
            origin,
            radius: 0.0,
            intensity,
            max_radius: 420.0,
        }
    }

    pub fn update(&mut self, dt: f32, config: &EffectConfig) {
        self.radius += config.shockwave_speed * dt;
    }

    pub fn opacity(&self) -> f32 {
        let t = (self.radius / self.max_radius).clamp(0.0, 1.0);
        (1.0 - t).powf(1.4) * self.intensity * 0.22
    }

    pub fn is_alive(&self) -> bool {
        self.radius < self.max_radius && self.opacity() > 0.005
    }
}

#[derive(Clone, Debug)]
pub struct ContactNormal {
    pub origin: Vec2,
    pub normal: Vec2,
    pub age: f32,
    pub lifetime: f32,
    pub length: f32,
}

impl ContactNormal {
    pub fn new(origin: Vec2, normal: Vec2, impact_speed: f32) -> Self {
        Self {
            origin,
            normal,
            age: 0.0,
            lifetime: 0.45,
            length: 18.0 + (impact_speed / 18.0).min(32.0),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.age += dt;
    }

    pub fn alpha(&self) -> f32 {
        (1.0 - self.age / self.lifetime).clamp(0.0, 1.0)
    }

    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }
}

fn push_capped<T>(items: &mut Vec<T>, cap: usize, item: T) {
    if cap == 0 {
        return;
    }
    if items.len() >= cap {
        items.swap_remove(0);
    }
    items.push(item);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shockwave_fades_as_it_expands() {
        let config = EffectConfig::default();
        let mut wave = Shockwave::new(Vec2::ZERO, 500.0);
        let first = wave.opacity();
        wave.update(0.2, &config);
        assert!(wave.opacity() < first);
    }
}
