// Import the macroquad game engine library
// 'use' brings items into scope, 'prelude::*' imports common types
use macroquad::prelude::*;
use std::f32::consts::PI;

// ============================================================================
// CONSTANTS - Easy to modify for different behaviors
// ============================================================================

// Physics constants - these control how the simulation feels
const GRAVITY: f32 = 800.0;          // Downward acceleration (pixels/second²)
const MIN_BALL_RADIUS: f32 = 4.0;    // Smallest ball size
const MAX_BALL_RADIUS: f32 = 10.0;   // Largest ball size
const MIN_VELOCITY: f32 = 50.0;      // Minimum speed to prevent balls getting stuck
const SUBSTEPS: usize = 8;           // Physics accuracy (more = better collision detection)
const AIR_DENSITY: f32 = 0.001;      // Air resistance factor (increase for "thicker" air)
const TEMPERATURE_DECAY: f32 = 0.98; // How fast temperature normalizes (0.98 = slow cooling)

// ============================================================================
// MATERIAL DEFINITIONS
// These define the physical properties of different ball materials
// ============================================================================

// Material properties for different ball types
const MATERIAL_RUBBER: Material = Material {
    density: 1.0,
    restitution: 0.85,
    friction: 0.8,
    drag_coefficient: 0.47,
    color_base: [0.8, 0.3, 0.3], // Reddish
};

const MATERIAL_STEEL: Material = Material {
    density: 3.0,
    restitution: 0.6,
    friction: 0.4,
    drag_coefficient: 0.4,
    color_base: [0.7, 0.7, 0.8], // Blueish gray
};

const MATERIAL_GLASS: Material = Material {
    density: 2.0,
    restitution: 0.95,
    friction: 0.2,
    drag_coefficient: 0.45,
    color_base: [0.6, 0.8, 0.9], // Light blue
};

// ============================================================================
// VISUAL EFFECTS CONSTANTS
// These control the appearance of ripples, sound waves, and other effects
// ============================================================================

const RIPPLE_BASE_SPEED: f32 = 150.0;    // How fast ripples expand
const MIN_RIPPLE_RADIUS: f32 = 20.0;     // Smallest ripple size
const MAX_RIPPLE_RADIUS: f32 = 300.0;    // Largest ripple size
const SOUND_WAVE_SPEED: f32 = 340.0;     // Speed of sound visualization (matches real speed of sound!)
const SPIN_INDICATOR_LENGTH: f32 = 1.5;  // Length of spin indicator line (multiplier of radius)

// ============================================================================
// SIMULATION CONFIGURATION
// These control the overall simulation setup and complexity
// ============================================================================

const MIN_BALLS: usize = 3;              // Minimum number of balls
const MAX_BALLS: usize = 10;             // Maximum number of balls (keep low for performance)
const MIN_LAYERS: usize = 2;             // Minimum polygon layers
const MAX_LAYERS: usize = 4;             // Maximum polygon layers
const LAYER_SPACING: f32 = 70.0;         // Distance between layers (pixels)
const ROTATION_SPEED_BASE: f32 = 0.3;    // Base rotation speed (radians/second)
const SIDES_PER_LAYER: usize = 10;       // Decagon (10-sided polygon)

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Material properties that affect physics behavior
#[derive(Clone, Copy)]
struct Material {
    density: f32,              // Affects mass (higher = heavier)
    restitution: f32,          // Bounciness (0 = no bounce, 1 = perfect bounce)
    friction: f32,             // Surface friction (0 = ice, 1 = rubber)
    drag_coefficient: f32,     // Air resistance shape factor
    color_base: [f32; 3],      // Base RGB color for visual identification
}

/// Information about a collision event
#[derive(Clone)]
struct CollisionInfo {
    point: Vec2,               // Where the collision happened
    impact_velocity: f32,      // How hard the collision was
    normal: Vec2,              // Direction of collision force
    material1: Material,       // First object's material
    material2: Material,       // Second object's material
}

/// A visual ripple effect that appears when balls hit walls
struct Ripple {
    origin: Vec2,              // Center point of the ripple
    radius: f32,               // Current size
    max_radius: f32,           // Maximum size before it disappears
    opacity: f32,              // Current transparency (0.0 = invisible, 1.0 = opaque)
    speed: f32,                // How fast it expands
    intensity: f32,            // Normalized impact strength (0.0-2.0)
    color: Color,              // Ripple color (based on material)
}

impl Ripple {
    /// Create a new ripple based on impact force and materials
    fn new(origin: Vec2, impact_velocity: f32, mat1: Material, mat2: Material, normal: Vec2) -> Self {
        // Normalize impact to a 0.2-2.0 range for visual scaling
        let normalized_impact = (impact_velocity / 500.0).clamp(0.2, 2.0);
        
        // Mix colors from both materials
        let color = Color::new(
            (mat1.color_base[0] + mat2.color_base[0]) * 0.5,
            (mat1.color_base[1] + mat2.color_base[1]) * 0.5,
            (mat1.color_base[2] + mat2.color_base[2]) * 0.5,
            1.0
        );
        
        // Offset origin slightly along normal for visual effect
        let offset_origin = origin + normal * 2.0;
        
        Ripple {
            origin: offset_origin,
            radius: 0.0,  // Starts at zero size
            max_radius: MIN_RIPPLE_RADIUS + (MAX_RIPPLE_RADIUS - MIN_RIPPLE_RADIUS) * normalized_impact.min(1.0),
            opacity: 0.4 + 0.4 * normalized_impact.min(1.0),
            speed: RIPPLE_BASE_SPEED * (0.8 + 0.4 * normalized_impact),
            intensity: normalized_impact,
            color,
        }
    }

    /// Update ripple animation each frame
    fn update(&mut self, dt: f32) {
        // Expand outward
        self.radius += self.speed * dt;
        
        // Fade out as it expands (like real water ripples)
        let fade_factor = 1.0 - (self.radius / self.max_radius).clamp(0.0, 1.0);
        self.opacity = fade_factor * (0.4 + 0.4 * self.intensity.min(1.0));
    }

    /// Check if this ripple should be removed
    fn is_alive(&self) -> bool {
        self.radius < self.max_radius && self.opacity > 0.01
    }
}

/// Sound wave visualization (travels faster than ripples)
struct SoundWave {
    origin: Vec2,
    radius: f32,
    intensity: f32,
    max_radius: f32,
}

impl SoundWave {
    fn new(origin: Vec2, intensity: f32) -> Self {
        SoundWave {
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
    
    fn get_opacity(&self) -> f32 {
        (1.0 - self.radius / self.max_radius) * self.intensity * 0.3
    }
}

/// A bouncing ball with advanced physics simulation
#[derive(Clone)]
struct Ball {
    // Position and motion
    pos: Vec2,                 // Current position
    vel: Vec2,                 // Current velocity (speed and direction)
    radius: f32,               // Size of the ball
    mass: f32,                 // Mass (calculated from radius and density)
    
    // Rotation
    angle: f32,                // Current rotation angle
    angular_velocity: f32,     // How fast it's spinning (radians/second)
    
    // Material and appearance
    material: Material,        // Physical properties
    color: Color,              // Visual color
    temperature: f32,          // Current temperature (1.0 = normal, >1 = hot)
    
    // Visual effects
    trail: Vec<Vec2>,          // History of positions for motion blur
}

impl Ball {
    /// Create a new ball with random properties
    fn new(x: f32, y: f32) -> Self {
        // Random size
        let radius = rand::gen_range(MIN_BALL_RADIUS, MAX_BALL_RADIUS);
        
        // Random material
        let material = match rand::gen_range(0, 3) {
            0 => MATERIAL_RUBBER,
            1 => MATERIAL_STEEL,
            _ => MATERIAL_GLASS,
        };
        
        // Calculate mass from volume and density
        // Mass = density * volume (using 2D area as proxy)
        let mass = material.density * PI * radius * radius / 100.0;
        
        // Base color with slight variation
        let color_variation = rand::gen_range(0.9, 1.1);
        let color = Color::new(
            (material.color_base[0] * color_variation).min(1.0),
            (material.color_base[1] * color_variation).min(1.0),
            (material.color_base[2] * color_variation).min(1.0),
            1.0
        );
        
        Ball {
            pos: Vec2::new(x, y),
            vel: Vec2::new(
                rand::gen_range(-400.0, 400.0),    // Random horizontal velocity
                rand::gen_range(-300.0, -100.0)     // Upward velocity (negative = up)
            ),
            radius,
            mass,
            angle: 0.0,
            angular_velocity: rand::gen_range(-5.0, 5.0), // Random initial spin
            material,
            color,
            temperature: 1.0, // Normal temperature
            trail: Vec::new(),
        }
    }

    /// Update ball physics for one frame (without ball-to-ball collision detection)
    fn update(&mut self, dt: f32, layers: &[PolygonLayer]) -> Vec<CollisionInfo> {
        let mut collisions = Vec::new();
        
        // Apply gravity (F = ma, so a = F/m)
        self.vel.y += GRAVITY * dt;
        
        // Apply air resistance (drag force)
        // F_drag = 0.5 * ρ * v² * C_d * A
        let speed = self.vel.length();
        if speed > 0.01 {
            let drag_force = 0.5 * AIR_DENSITY * speed * speed * self.material.drag_coefficient * self.radius;
            let drag_acceleration = drag_force / self.mass;
            let drag_direction = -self.vel.normalize();
            self.vel += drag_direction * drag_acceleration * dt;
        }
        
        // Apply angular air resistance (rotational drag)
        self.angular_velocity *= 0.99;
        
        // Temperature decay (return to normal)
        self.temperature += (1.0 - self.temperature) * (1.0 - TEMPERATURE_DECAY) * dt * 10.0;
        
        // Update rotation
        self.angle += self.angular_velocity * dt;
        
        // Break update into smaller steps for accurate collision detection
        let sub_dt = dt / SUBSTEPS as f32;
        
        for _ in 0..SUBSTEPS {
            let old_pos = self.pos;
            
            // Move based on velocity
            self.pos += self.vel * sub_dt;
            
            // Check collision with walls
            let mut collision_found = false;
            for layer in layers {
                if let Some((contact_point, normal)) = layer.check_collision(&self) {
                    let impact_velocity = self.vel.dot(-normal).abs();
                    
                    // Create wall material (hard surface)
                    let wall_material = Material {
                        density: 10.0,
                        restitution: 0.8,
                        friction: 0.5,
                        drag_coefficient: 0.0,
                        color_base: [0.8, 0.8, 0.8],
                    };
                    
                    collisions.push(CollisionInfo {
                        point: contact_point,
                        impact_velocity,
                        normal,
                        material1: self.material,
                        material2: wall_material,
                    });
                    
                    // Restore position
                    self.pos = old_pos;
                    
                    // Combined restitution (average of both materials)
                    let combined_restitution = (self.material.restitution + wall_material.restitution) * 0.5;
                    
                    // Reflect velocity
                    self.vel = reflect_velocity(self.vel, normal, combined_restitution);
                    
                    // Apply friction to spin
                    let tangent = Vec2::new(-normal.y, normal.x);
                    let relative_velocity = self.vel.dot(tangent) - self.angular_velocity * self.radius;
                    let friction_impulse = relative_velocity * self.material.friction * 0.1;
                    self.angular_velocity += friction_impulse / self.radius;
                    
                    // Heat from impact
                    self.temperature += impact_velocity * 0.001;
                    self.temperature = self.temperature.min(3.0); // Cap temperature
                    
                    // Ensure minimum velocity
                    let speed = self.vel.length();
                    if speed < MIN_VELOCITY {
                        self.vel = self.vel.normalize_or_zero() * MIN_VELOCITY;
                    }
                    
                    // Move away from wall
                    self.pos = contact_point + normal * (self.radius + 0.5);
                    
                    collision_found = true;
                    break;
                }
            }
            
            if collision_found {
                break;
            }
        }
        
        // Update motion trail for visual effect
        self.trail.push(self.pos);
        if self.trail.len() > 15 {
            self.trail.remove(0);
        }
        
        collisions
    }

    /// Resolve collision with another ball using conservation of momentum
    /// This implements the physics equations for elastic collisions between two spheres
    fn resolve_ball_collision(&mut self, other: &mut Ball, normal: Vec2, combined_restitution: f32) {
        // Calculate relative velocity along the collision normal
        let relative_velocity = self.vel - other.vel;
        let velocity_along_normal = relative_velocity.dot(normal);
        
        // Don't resolve if velocities are separating (balls moving apart)
        if velocity_along_normal > 0.0 {
            return;
        }
        
        // Calculate impulse magnitude using conservation of momentum
        // J = (2 * m1 * m2 * |v_rel|) / (m1 + m2)
        let impulse = 2.0 * velocity_along_normal / (1.0 / self.mass + 1.0 / other.mass);
        let impulse_vector = impulse * normal * combined_restitution;
        
        // Apply impulse to velocities (Newton's third law: equal and opposite reactions)
        self.vel -= impulse_vector / self.mass;
        other.vel += impulse_vector / other.mass;
        
        // Transfer some linear momentum to angular momentum (creates spin)
        let tangent = Vec2::new(-normal.y, normal.x);
        let self_tangent_vel = self.vel.dot(tangent);
        let other_tangent_vel = other.vel.dot(tangent);
        
        // Apply friction to create spin (torque = force × radius)
        self.angular_velocity -= self_tangent_vel * self.material.friction * 0.05 / self.radius;
        other.angular_velocity += other_tangent_vel * other.material.friction * 0.05 / other.radius;
        
        // Generate heat from collision (kinetic energy → thermal energy)
        let impact_energy = impulse.abs() * 0.001;
        self.temperature += impact_energy;
        other.temperature += impact_energy;
        
        // Separate balls to prevent overlap (position correction)
        let distance = (self.pos - other.pos).length();
        let overlap = (self.radius + other.radius) - distance;
        if overlap > 0.0 {
            let separation = normal * (overlap * 0.5 + 0.1);
            self.pos += separation;
            other.pos -= separation;
        }
    }

    /// Draw the ball with all visual effects
    fn draw(&self) {
        // Draw motion trail (older positions fade out)
        for (i, &pos) in self.trail.iter().enumerate() {
            let alpha = (i as f32 / self.trail.len() as f32) * 0.3;
            let mut trail_color = self.color;
            trail_color.a = alpha;
            draw_circle(pos.x, pos.y, self.radius * 0.5, trail_color);
        }
        
        // Temperature affects color - balls glow red when hot from collisions
        // This simulates thermal energy from impacts
        let temp_color = Color::new(
            (self.color.r * self.temperature).min(1.0),      // Red channel increases with heat
            self.color.g / self.temperature.sqrt(),           // Green decreases
            self.color.b / self.temperature,                  // Blue decreases more
            self.color.a
        );
        
        // Draw main ball
        draw_circle(self.pos.x, self.pos.y, self.radius, temp_color);
        
        // Draw spin indicator - shows rotation speed and direction
        if self.angular_velocity.abs() > 0.1 {
            let indicator_end = self.pos + Vec2::new(
                self.angle.cos() * self.radius * SPIN_INDICATOR_LENGTH,
                self.angle.sin() * self.radius * SPIN_INDICATOR_LENGTH
            );
            draw_line(
                self.pos.x, self.pos.y,
                indicator_end.x, indicator_end.y,
                1.5,
                Color::new(1.0, 1.0, 1.0, 0.5)
            );
        }
        
        // Draw 3D-style highlight for depth
        let highlight = Color::new(
            (temp_color.r + 0.3).min(1.0),
            (temp_color.g + 0.3).min(1.0),
            (temp_color.b + 0.3).min(1.0),
            0.6
        );
        draw_circle(
            self.pos.x - self.radius * 0.3,
            self.pos.y - self.radius * 0.3,
            self.radius * 0.5,
            highlight
        );
        
        // Draw material density indicator (small colored dot in center)
        // Green = light, Yellow = medium, Red = heavy
        let material_indicator_color = match self.material.density {
            d if d < 1.5 => GREEN,   // Light material (rubber)
            d if d < 2.5 => YELLOW,  // Medium material (glass)
            _ => RED,                // Heavy material (steel)
        };
        draw_circle(self.pos.x, self.pos.y, 2.0, material_indicator_color);
    }
}

/// A rotating polygon layer that balls can bounce off
struct PolygonLayer {
    center: Vec2,                    // Center of rotation
    base_vertices: Vec<Vec2>,        // Vertex positions (before rotation)
    active_edges: Vec<bool>,         // Which edges exist (false = gap)
    rotation: f32,                   // Current rotation angle (radians)
    rotation_speed: f32,             // How fast it rotates (radians/second)
    layer_index: usize,              // 0 = outermost, higher = inner
    color: Color,                    // Display color
}

impl PolygonLayer {
    /// Create a new polygon layer
    fn new(center: Vec2, radius: f32, layer_index: usize, total_layers: usize) -> Self {
        // Create vertices for a regular decagon
        let mut vertices = Vec::new();
        for i in 0..SIDES_PER_LAYER {
            let angle = (i as f32) * 2.0 * PI / SIDES_PER_LAYER as f32 - PI / SIDES_PER_LAYER as f32;
            vertices.push(Vec2::new(
                angle.cos() * radius,
                angle.sin() * radius,
            ));
        }
        
        // Start with all edges active
        let mut active_edges = vec![true; SIDES_PER_LAYER];
        
        // Remove 2-3 random edges for inner layers
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
        
        // Randomize rotation
        let clockwise = rand::gen_range(0, 2) == 0;
        let speed_variation = rand::gen_range(0.5, 1.5);
        let rotation_speed = ROTATION_SPEED_BASE * speed_variation * if clockwise { 1.0 } else { -1.0 };
        
        // Inner layers are darker
        let brightness = 0.8 - (layer_index as f32 / total_layers as f32) * 0.4;
        let color = Color::new(brightness, brightness, brightness * 1.1, 1.0);
        
        PolygonLayer {
            center,
            base_vertices: vertices,
            active_edges,
            rotation: rand::gen_range(0.0, 2.0 * PI),
            rotation_speed,
            layer_index,
            color,
        }
    }

    fn update(&mut self, dt: f32) {
        self.rotation += self.rotation_speed * dt;
    }

    fn get_transformed_vertices(&self) -> Vec<Vec2> {
        self.base_vertices
            .iter()
            .map(|v| {
                let rotated = rotate_point(*v, self.rotation);
                rotated + self.center
            })
            .collect()
    }

    fn draw(&self) {
        let transformed = self.get_transformed_vertices();
        let n = transformed.len();
        
        // Only fill the outermost layer
        if self.layer_index == 0 {
            for i in 0..n {
                if self.active_edges[i] {
                    let v1 = transformed[i];
                    let v2 = transformed[(i + 1) % n];
                    draw_triangle(
                        self.center,
                        v1,
                        v2,
                        Color::new(0.1, 0.15, 0.2, 0.3)
                    );
                }
            }
        }
        
        // Draw edges
        for i in 0..n {
            if self.active_edges[i] {
                let v1 = transformed[i];
                let v2 = transformed[(i + 1) % n];
                let thickness = 3.0 - self.layer_index as f32 * 0.5;
                draw_line(v1.x, v1.y, v2.x, v2.y, thickness, self.color);
            }
        }
        
        // Draw vertices
        for i in 0..n {
            let prev_edge = if i == 0 { n - 1 } else { i - 1 };
            if self.active_edges[i] || self.active_edges[prev_edge] || self.active_edges[(i + 1) % n] {
                let v = transformed[i];
                let size = 2.5 - self.layer_index as f32 * 0.3;
                draw_circle(v.x, v.y, size, YELLOW);
            }
        }
    }

    fn draw_ripple(&self, ripple: &Ripple) {
        let rings = (2.0 + ripple.intensity * 2.0) as i32;
        
        for i in 0..rings {
            let offset = i as f32 * (8.0 + 4.0 * (1.0 - ripple.intensity));
            let radius = ripple.radius - offset;
            
            if radius > 0.0 && radius < ripple.max_radius {
                let ring_opacity = ripple.opacity * (1.0 - i as f32 / rings as f32);
                
                let segments = 60;
                for j in 0..segments {
                    let angle1 = (j as f32) * 2.0 * PI / segments as f32;
                    let angle2 = ((j + 1) as f32) * 2.0 * PI / segments as f32;
                    
                    let p1 = ripple.origin + Vec2::new(angle1.cos() * radius, angle1.sin() * radius);
                    let p2 = ripple.origin + Vec2::new(angle2.cos() * radius, angle2.sin() * radius);
                    
                    if self.point_inside(&p1) && self.point_inside(&p2) {
                        let thickness = (1.5 + ripple.intensity) - i as f32 * 0.5;
                        let mut color = ripple.color;
                        color.a = ring_opacity;
                        draw_line(p1.x, p1.y, p2.x, p2.y, thickness, color);
                    }
                }
            }
        }
    }

    fn draw_sound_wave(&self, wave: &SoundWave) {
        let opacity = wave.get_opacity();
        if opacity > 0.01 {
            let segments = 60;
            for j in 0..segments {
                let angle1 = (j as f32) * 2.0 * PI / segments as f32;
                let angle2 = ((j + 1) as f32) * 2.0 * PI / segments as f32;
                
                let p1 = wave.origin + Vec2::new(angle1.cos() * wave.radius, angle1.sin() * wave.radius);
                let p2 = wave.origin + Vec2::new(angle2.cos() * wave.radius, angle2.sin() * wave.radius);
                
                if self.point_inside(&p1) && self.point_inside(&p2) {
                    draw_line(
                        p1.x, p1.y, p2.x, p2.y, 
                        1.0, 
                        Color::new(1.0, 1.0, 1.0, opacity)
                    );
                }
            }
        }
    }

    fn check_collision(&self, ball: &Ball) -> Option<(Vec2, Vec2)> {
        let transformed = self.get_transformed_vertices();
        let n = transformed.len();
        
        if self.layer_index > 0 && !self.point_inside(&ball.pos) {
            return None;
        }
        
        if self.layer_index == 0 && !self.point_inside(&ball.pos) {
            return None;
        }
        
        let mut closest_collision: Option<(Vec2, Vec2, f32)> = None;
        
        for i in 0..n {
            if !self.active_edges[i] {
                continue;
            }
            
            let v1 = transformed[i];
            let v2 = transformed[(i + 1) % n];
            
            if let Some((point, normal, distance)) = circle_line_collision(ball.pos, ball.radius, v1, v2) {
                if closest_collision.is_none() || distance < closest_collision.as_ref().unwrap().2 {
                    closest_collision = Some((point, normal, distance));
                }
            }
        }
        
        closest_collision.map(|(point, normal, _)| (point, normal))
    }

    fn point_inside(&self, point: &Vec2) -> bool {
        let transformed = self.get_transformed_vertices();
        let n = transformed.len();
        let mut inside = false;

        for i in 0..n {
            if !self.active_edges[i] {
                continue;
            }
            
            let v1 = transformed[i];
            let v2 = transformed[(i + 1) % n];
            
            if ((v1.y > point.y) != (v2.y > point.y)) &&
               (point.x < (v2.x - v1.x) * (point.y - v1.y) / (v2.y - v1.y) + v1.x) {
                inside = !inside;
            }
        }

        inside
    }
}

// ============================================================================
// PHYSICS HELPER FUNCTIONS
// ============================================================================

fn rotate_point(point: Vec2, angle: f32) -> Vec2 {
    Vec2::new(
        point.x * angle.cos() - point.y * angle.sin(),
        point.x * angle.sin() + point.y * angle.cos(),
    )
}

fn circle_line_collision(circle_pos: Vec2, radius: f32, line_start: Vec2, line_end: Vec2) -> Option<(Vec2, Vec2, f32)> {
    let line_vec = line_end - line_start;
    let circle_to_line_start = circle_pos - line_start;
    
    let line_length_sq = line_vec.dot(line_vec);
    if line_length_sq == 0.0 {
        return None;
    }
    
    let t = (circle_to_line_start.dot(line_vec) / line_length_sq).clamp(0.0, 1.0);
    let closest_point = line_start + line_vec * t;
    
    let distance = (circle_pos - closest_point).length();
    
    if distance <= radius {
        let normal = if distance > 0.001 {
            (circle_pos - closest_point).normalize()
        } else {
            Vec2::new(-line_vec.y, line_vec.x).normalize()
        };
        
        Some((closest_point, normal, distance))
    } else {
        None
    }
}

fn reflect_velocity(velocity: Vec2, normal: Vec2, restitution: f32) -> Vec2 {
    let dot = velocity.dot(normal);
    
    if dot > 0.0 {
        return velocity;
    }
    
    velocity - normal * (2.0 * dot * restitution)
}

// ============================================================================
// MAIN SIMULATION CONTROLLER
// ============================================================================

struct Simulation {
    balls: Vec<Ball>,
    layers: Vec<PolygonLayer>,
    ripples: Vec<Ripple>,
    sound_waves: Vec<SoundWave>,
    show_info: bool,
    total_collisions: usize,
    total_energy: f32,
}

impl Simulation {
    fn new() -> Self {
        let center = Vec2::new(screen_width() / 2.0, screen_height() / 2.0);
        
        // Spawn random number of balls
        let num_balls = rand::gen_range(MIN_BALLS, MAX_BALLS + 1);
        let mut balls = Vec::new();
        
        for i in 0..num_balls {
            let angle = (i as f32) * 2.0 * PI / num_balls as f32;
            let spawn_radius = 100.0 + rand::gen_range(-20.0, 20.0);
            let x = center.x + angle.cos() * spawn_radius;
            let y = center.y + angle.sin() * spawn_radius - 50.0;
            balls.push(Ball::new(x, y));
        }
        
        // Create layers
        let num_layers = rand::gen_range(MIN_LAYERS, MAX_LAYERS + 1);
        let mut layers = Vec::new();
        
        for i in 0..num_layers {
            let radius = 280.0 - i as f32 * LAYER_SPACING;
            layers.push(PolygonLayer::new(center, radius, i, num_layers));
        }
        
        Simulation {
            balls,
            layers,
            ripples: Vec::new(),
            sound_waves: Vec::new(),
            show_info: true,
            total_collisions: 0,
            total_energy: 0.0,
        }
    }

    fn update(&mut self, dt: f32) {
        // Update layers
        for layer in &mut self.layers {
            layer.update(dt);
        }
        
        // Update balls and collect wall collisions
        let mut all_collisions = Vec::new();
        
        // First pass: update physics and detect wall collisions
        for ball in &mut self.balls {
            let collisions = ball.update(dt, &self.layers);
            all_collisions.extend(collisions);
        }
        
        // Second pass: detect and resolve ball-to-ball collisions
        for i in 0..self.balls.len() {
            for j in i + 1..self.balls.len() {
                let distance = (self.balls[i].pos - self.balls[j].pos).length();
                let min_distance = self.balls[i].radius + self.balls[j].radius;
                
                if distance < min_distance && distance > 0.01 {
                    let normal = (self.balls[i].pos - self.balls[j].pos).normalize();
                    let relative_velocity = self.balls[i].vel - self.balls[j].vel;
                    let impact_speed = relative_velocity.dot(normal);
                    
                    if impact_speed < 0.0 {
                        // Store collision info before resolution
                        let contact_point = self.balls[i].pos - normal * (self.balls[i].radius - (min_distance - distance) * 0.5);
                        all_collisions.push(CollisionInfo {
                            point: contact_point,
                            impact_velocity: impact_speed.abs(),
                            normal,
                            material1: self.balls[i].material,
                            material2: self.balls[j].material,
                        });
                        
                        // Split the mutable borrow
                        let (left, right) = self.balls.split_at_mut(j);
                        let ball1 = &mut left[i];
                        let ball2 = &mut right[0];
                        
                        // Combined material properties
                        let combined_restitution = (ball1.material.restitution + ball2.material.restitution) * 0.5;
                        
                        // Resolve collision
                        ball1.resolve_ball_collision(ball2, normal, combined_restitution);
                    }
                }
            }
        }
        
        // Create effects for collisions
        for collision in all_collisions {
            self.ripples.push(Ripple::new(
                collision.point, 
                collision.impact_velocity,
                collision.material1,
                collision.material2,
                collision.normal
            ));
            
            // Create sound wave for loud collisions
            if collision.impact_velocity > 200.0 {
                self.sound_waves.push(SoundWave::new(
                    collision.point,
                    collision.impact_velocity / 1000.0
                ));
            }
            
            self.total_collisions += 1;
        }
        
        // Update effects
        for ripple in &mut self.ripples {
            ripple.update(dt);
        }
        
        for wave in &mut self.sound_waves {
            wave.update(dt);
        }
        
        // Remove dead effects
        self.ripples.retain(|r| r.is_alive());
        self.sound_waves.retain(|w| w.is_alive());
        
        // Calculate total energy
        self.total_energy = self.balls.iter()
            .map(|b| 0.5 * b.mass * b.vel.length_squared() + // Kinetic energy
                    0.5 * b.mass * b.radius * b.radius * b.angular_velocity * b.angular_velocity) // Rotational energy
            .sum();
    }

    fn draw(&self) {
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
        
        // Draw effects in the right order
        for layer in self.layers.iter().rev() {
            if layer.layer_index == 0 {
                // Draw ripples
                for ripple in &self.ripples {
                    layer.draw_ripple(ripple);
                }
                
                // Draw sound waves
                for wave in &self.sound_waves {
                    layer.draw_sound_wave(wave);
                }
            }
        }
        
        // Draw layers
        for layer in &self.layers {
            layer.draw();
        }
        
        // Draw balls
        for ball in &self.balls {
            ball.draw();
        }
        
        if self.show_info {
            self.draw_info();
        }
    }

    /// Draw debug information and material legend
    fn draw_info(&self) {
        let info_color = GREEN;
        
        // Performance and stats
        draw_text(&format!("FPS: {}", get_fps()), 10.0, 20.0, 20.0, info_color);
        draw_text(
            &format!("Balls: {} (R={} S={} G={})", 
                self.balls.len(),
                self.balls.iter().filter(|b| b.material.density < 1.5).count(),
                self.balls.iter().filter(|b| b.material.density >= 1.5 && b.material.density < 2.5).count(),
                self.balls.iter().filter(|b| b.material.density >= 2.5).count()
            ),
            10.0, 40.0, 20.0, info_color
        );
        draw_text(
            &format!("Total Collisions: {}", self.total_collisions),
            10.0, 60.0, 20.0, info_color
        );
        draw_text(
            &format!("System Energy: {:.0} J", self.total_energy),
            10.0, 80.0, 20.0, info_color
        );
        draw_text(
            "Effects: Ripples + Sound Waves",
            10.0, 100.0, 20.0, info_color
        );
        
        // Material legend with visual indicators
        draw_text("Materials:", 10.0, 130.0, 16.0, GRAY);
        
        // Rubber
        draw_circle(20.0, 150.0, 5.0, Color::new(0.8, 0.3, 0.3, 1.0));
        draw_text("Rubber: Light, Bouncy, High Friction", 35.0, 155.0, 14.0, GRAY);
        
        // Steel
        draw_circle(20.0, 170.0, 5.0, Color::new(0.7, 0.7, 0.8, 1.0));
        draw_text("Steel: Heavy, Less Bouncy, Smooth", 35.0, 175.0, 14.0, GRAY);
        
        // Glass
        draw_circle(20.0, 190.0, 5.0, Color::new(0.6, 0.8, 0.9, 1.0));
        draw_text("Glass: Medium, Very Bouncy, Slippery", 35.0, 195.0, 14.0, GRAY);
        
        // Controls
        draw_text(
            "Controls: SPACE = Toggle Info | R = New Simulation",
            10.0, 220.0, 20.0, GRAY
        );
    }

    fn handle_input(&mut self) {
        if is_key_pressed(KeyCode::Space) {
            self.show_info = !self.show_info;
        }
        
        if is_key_pressed(KeyCode::R) {
            *self = Simulation::new();
        }
    }
}

#[macroquad::main("Advanced Physics Simulation")]
async fn main() {
    rand::srand(macroquad::miniquad::date::now() as _);
    let mut sim = Simulation::new();
    
    loop {
        let dt = get_frame_time().min(0.016);
        
        sim.handle_input();
        sim.update(dt);
        sim.draw();
        
        next_frame().await
    }
}