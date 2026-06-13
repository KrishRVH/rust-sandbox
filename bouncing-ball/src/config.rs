use macroquad::prelude::Vec2;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub physics: PhysicsConfig,
    pub spawn: SpawnConfig,
    pub arena: ArenaConfig,
    pub effects: EffectConfig,
    pub render: RenderConfig,
}

#[derive(Clone, Debug)]
pub struct PhysicsConfig {
    pub fixed_dt: f32,
    pub max_frame_time: f32,
    pub max_steps_per_frame: usize,
    pub gravity: f32,
    pub substeps: usize,
    pub solver_iterations: usize,
    pub air_density: f32,
    pub angular_damping_per_sec: f32,
    pub heat_decay_per_sec: f32,
    pub vortex_drive: f32,
    pub recenter_drive: f32,
    pub restitution_slop: f32,
    pub penetration_slop: f32,
    pub position_correction_percent: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            fixed_dt: 1.0 / 120.0,
            max_frame_time: 0.25,
            max_steps_per_frame: 5,
            gravity: -180.0,
            substeps: 6,
            solver_iterations: 3,
            air_density: 0.00055,
            angular_damping_per_sec: 1.35,
            heat_decay_per_sec: 1.6,
            vortex_drive: 520.0,
            recenter_drive: 520.0,
            restitution_slop: 18.0,
            penetration_slop: 0.03,
            position_correction_percent: 0.82,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpawnConfig {
    pub min_balls: usize,
    pub max_balls: usize,
    pub min_radius: f32,
    pub max_radius: f32,
    pub launch_speed_min: f32,
    pub launch_speed_max: f32,
    pub trail_length: usize,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            min_balls: 10,
            max_balls: 12,
            min_radius: 10.0,
            max_radius: 16.0,
            launch_speed_min: 95.0,
            launch_speed_max: 240.0,
            trail_length: 28,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArenaConfig {
    pub min_layers: usize,
    pub max_layers: usize,
    pub sides_per_layer: usize,
    pub outer_radius: f32,
    pub layer_spacing: f32,
    pub rotation_speed_base: f32,
}

impl Default for ArenaConfig {
    fn default() -> Self {
        Self {
            min_layers: 3,
            max_layers: 5,
            sides_per_layer: 12,
            outer_radius: 350.0,
            layer_spacing: 68.0,
            rotation_speed_base: 0.22,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EffectConfig {
    pub max_ripples: usize,
    pub max_shockwaves: usize,
    pub max_normals: usize,
    pub ripple_base_speed: f32,
    pub ripple_min_radius: f32,
    pub ripple_max_radius: f32,
    pub shockwave_speed: f32,
}

impl Default for EffectConfig {
    fn default() -> Self {
        Self {
            max_ripples: 36,
            max_shockwaves: 10,
            max_normals: 64,
            ripple_base_speed: 170.0,
            ripple_min_radius: 18.0,
            ripple_max_radius: 260.0,
            shockwave_speed: 380.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderConfig {
    pub virtual_width: f32,
    pub virtual_height: f32,
    pub ring_segments: usize,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            virtual_width: 1280.0,
            virtual_height: 820.0,
            ring_segments: 32,
        }
    }
}

impl RenderConfig {
    pub fn world_size(&self) -> Vec2 {
        Vec2::new(self.virtual_width, self.virtual_height)
    }

    pub fn world_center(&self) -> Vec2 {
        self.world_size() * 0.5
    }
}

impl Config {
    pub fn normalized(mut self) -> Self {
        self.physics.fixed_dt = self.physics.fixed_dt.max(1.0 / 1000.0);
        self.physics.max_frame_time = self.physics.max_frame_time.max(self.physics.fixed_dt);
        self.physics.max_steps_per_frame = self.physics.max_steps_per_frame.max(1);
        self.physics.substeps = self.physics.substeps.max(1);
        self.physics.solver_iterations = self.physics.solver_iterations.max(1);
        self.physics.penetration_slop = self.physics.penetration_slop.max(0.0);
        self.physics.position_correction_percent =
            self.physics.position_correction_percent.clamp(0.0, 1.0);

        self.spawn.min_balls = self.spawn.min_balls.max(1);
        self.spawn.max_balls = self.spawn.max_balls.max(self.spawn.min_balls);
        self.spawn.min_radius = self.spawn.min_radius.max(2.0);
        self.spawn.max_radius = self.spawn.max_radius.max(self.spawn.min_radius);
        self.spawn.launch_speed_min = self.spawn.launch_speed_min.max(0.0);
        self.spawn.launch_speed_max = self.spawn.launch_speed_max.max(self.spawn.launch_speed_min);
        self.spawn.trail_length = self.spawn.trail_length.max(2);

        self.arena.min_layers = self.arena.min_layers.max(1);
        self.arena.max_layers = self.arena.max_layers.max(self.arena.min_layers);
        self.arena.sides_per_layer = self.arena.sides_per_layer.max(4);
        self.arena.outer_radius = self.arena.outer_radius.max(self.spawn.max_radius * 8.0);
        self.arena.layer_spacing = self.arena.layer_spacing.max(self.spawn.max_radius * 2.0);

        self.effects.max_normals = self.effects.max_normals.max(1);
        self.render.virtual_width = self.render.virtual_width.max(320.0);
        self.render.virtual_height = self.render.virtual_height.max(240.0);
        self.render.ring_segments = self.render.ring_segments.max(12);

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_config_clamps_invalid_public_values() {
        let mut config = Config::default();
        config.spawn.min_balls = 0;
        config.spawn.max_balls = 0;
        config.spawn.trail_length = 0;
        config.arena.sides_per_layer = 3;
        config.physics.substeps = 0;
        config.physics.solver_iterations = 0;

        let config = config.normalized();

        assert_eq!(config.spawn.min_balls, 1);
        assert_eq!(config.spawn.max_balls, 1);
        assert_eq!(config.spawn.trail_length, 2);
        assert_eq!(config.arena.sides_per_layer, 4);
        assert_eq!(config.physics.substeps, 1);
        assert_eq!(config.physics.solver_iterations, 1);
    }
}
