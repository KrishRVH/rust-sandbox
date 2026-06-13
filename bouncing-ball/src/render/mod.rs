use macroquad::prelude::*;

use crate::config::RenderConfig;
use crate::input::ViewOptions;
use crate::sim::{
    Arena, ArenaLayer, Ball, ContactNormal, Effects, MaterialKind, Rgb, Ripple, Shockwave, World,
};

#[derive(Clone, Copy, Debug)]
pub struct OverlayState {
    pub fps: i32,
    pub fixed_hz: u32,
    pub substeps: usize,
    pub last_steps: usize,
    pub dropped_time: f32,
    pub time_scale: f32,
    pub paused: bool,
}

#[derive(Clone, Copy, Debug)]
struct Viewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scale: f32,
}

#[derive(Clone, Copy, Debug)]
struct Palette {
    base: Color,
    rim: Color,
    highlight: Color,
    shadow: Color,
}

const BACKGROUND: Color = Color::new(0.035, 0.040, 0.055, 1.0);
const PANEL: Color = Color::new(0.025, 0.030, 0.040, 0.74);
const TEXT: Color = Color::new(0.78, 0.83, 0.88, 1.0);
const MUTED_TEXT: Color = Color::new(0.48, 0.53, 0.60, 1.0);
const ACCENT: Color = Color::new(0.50, 0.88, 1.0, 1.0);
const WARNING: Color = Color::new(1.0, 0.67, 0.26, 1.0);

pub fn draw(world: &World, view: &ViewOptions, overlay: OverlayState) {
    clear_background(BACKGROUND);

    let viewport = letterbox_viewport(&world.config.render);
    draw_letterbox(&viewport);
    set_camera(&world_camera(&world.config.render, viewport));

    draw_background_grid(world);
    draw_vortex_field(world);

    if view.effects {
        draw_effects(&world.effects, &world.config.render);
    }

    draw_arena(&world.arena);

    for ball in &world.balls {
        draw_ball(ball, view);
    }

    if view.velocity_vectors {
        draw_velocity_vectors(&world.balls);
    }

    if view.collision_normals {
        draw_contact_normals(&world.effects.normals);
    }

    set_default_camera();

    if view.compact_hud {
        draw_compact_overlay(world, overlay);
    }

    if view.help_overlay {
        draw_help_overlay(world);
    }

    draw_hover_inspector(world, viewport);
    draw_control_hint();
}

fn letterbox_viewport(config: &RenderConfig) -> Viewport {
    let scale =
        (screen_width() / config.virtual_width).min(screen_height() / config.virtual_height);
    let width = config.virtual_width * scale;
    let height = config.virtual_height * scale;
    Viewport {
        x: (screen_width() - width) * 0.5,
        y: (screen_height() - height) * 0.5,
        width,
        height,
        scale,
    }
}

fn world_camera(config: &RenderConfig, viewport: Viewport) -> Camera2D {
    Camera2D {
        target: config.world_center(),
        zoom: Vec2::new(2.0 / config.virtual_width, -2.0 / config.virtual_height),
        viewport: Some((
            viewport.x.round() as i32,
            viewport.y.round() as i32,
            viewport.width.round() as i32,
            viewport.height.round() as i32,
        )),
        ..Default::default()
    }
}

fn draw_letterbox(viewport: &Viewport) {
    let border = Color::new(0.11, 0.12, 0.14, 1.0);
    draw_rectangle_lines(
        viewport.x - 1.0,
        viewport.y - 1.0,
        viewport.width + 2.0,
        viewport.height + 2.0,
        2.0,
        border,
    );
}

fn draw_background_grid(world: &World) {
    let width = world.config.render.virtual_width;
    let height = world.config.render.virtual_height;
    let grid = Color::new(0.12, 0.16, 0.19, 0.20);
    let major = Color::new(0.16, 0.21, 0.25, 0.22);

    let mut x = 0.0;
    while x <= width {
        let color = if (x as i32) % 160 == 0 { major } else { grid };
        draw_line(x, 0.0, x, height, 1.0, color);
        x += 40.0;
    }

    let mut y = 0.0;
    while y <= height {
        let color = if (y as i32) % 160 == 0 { major } else { grid };
        draw_line(0.0, y, width, y, 1.0, color);
        y += 40.0;
    }

    let center = world.config.render.world_center();
    draw_line(
        center.x,
        center.y - 42.0,
        center.x,
        center.y + 42.0,
        1.0,
        Color::new(0.42, 0.52, 0.62, 0.16),
    );
    draw_line(
        center.x - 42.0,
        center.y,
        center.x + 42.0,
        center.y,
        1.0,
        Color::new(0.42, 0.52, 0.62, 0.16),
    );
}

fn draw_vortex_field(world: &World) {
    let center = world.arena.center;
    let color = Color::new(0.28, 0.64, 0.82, 0.13);

    for radius in [54.0, 94.0, 138.0] {
        draw_circle_lines(center.x, center.y, radius, 1.0, color);
    }

    for index in 0..8 {
        let angle = index as f32 * std::f32::consts::TAU / 8.0 + get_time() as f32 * 0.45;
        let start = center + Vec2::new(angle.cos(), angle.sin()) * 62.0;
        let tangent = Vec2::new(-angle.sin(), angle.cos());
        let end = start + tangent * 28.0;
        draw_line(
            start.x,
            start.y,
            end.x,
            end.y,
            1.2,
            Color::new(0.42, 0.86, 1.0, 0.20),
        );
        draw_circle(end.x, end.y, 2.0, Color::new(0.42, 0.86, 1.0, 0.20));
    }
}

fn draw_arena(arena: &Arena) {
    for layer in &arena.layers {
        draw_arena_layer_fill(layer);
    }

    for layer in &arena.layers {
        draw_arena_layer_edges(layer);
    }
}

fn draw_arena_layer_fill(layer: &ArenaLayer) {
    if layer.index != 0 {
        return;
    }

    for i in 0..layer.world_vertices.len() {
        let Some((start, end)) = layer.active_segment(i) else {
            continue;
        };
        draw_triangle(
            layer.center,
            start,
            end,
            Color::new(0.055, 0.075, 0.095, 0.58),
        );
    }
}

fn draw_arena_layer_edges(layer: &ArenaLayer) {
    let depth = layer.index as f32;
    let edge_color = Color::new(
        (0.74 - depth * 0.07).max(0.42),
        (0.78 - depth * 0.07).max(0.46),
        (0.92 - depth * 0.06).max(0.58),
        0.82,
    );
    let glow_color = Color::new(0.38, 0.56, 0.74, 0.10);
    let thickness = (4.2 - depth * 0.45).max(1.6);

    for i in 0..layer.world_vertices.len() {
        let Some((start, end)) = layer.active_segment(i) else {
            continue;
        };
        draw_line(start.x, start.y, end.x, end.y, thickness + 5.0, glow_color);
        draw_line(start.x, start.y, end.x, end.y, thickness, edge_color);
    }

    for i in 0..layer.world_vertices.len() {
        let prev = if i == 0 {
            layer.world_vertices.len() - 1
        } else {
            i - 1
        };
        if layer.active_edges[i] || layer.active_edges[prev] {
            let vertex = layer.world_vertices[i];
            draw_circle(
                vertex.x,
                vertex.y,
                (3.0 - depth * 0.22).max(1.7),
                Color::new(0.98, 0.86, 0.38, 0.78),
            );
        }
    }
}

fn draw_effects(effects: &Effects, config: &RenderConfig) {
    for ripple in &effects.ripples {
        draw_ripple(ripple, config.ring_segments);
    }
    for shockwave in &effects.shockwaves {
        draw_shockwave(shockwave, config.ring_segments);
    }
}

fn draw_ripple(ripple: &Ripple, segments: usize) {
    let rings = if matches!(ripple.kind, crate::sim::collision::CollisionKind::BallBall) {
        2
    } else {
        3
    };

    for ring in 0..rings {
        let radius = ripple.radius - ring as f32 * 9.0;
        if radius <= 0.0 {
            continue;
        }
        let mut color = color_from_rgb(ripple.color, ripple.opacity * (1.0 - ring as f32 * 0.28));
        color.a *= 0.55;
        draw_ring(
            ripple.origin,
            radius,
            segments,
            1.2 + ripple.intensity * 0.55,
            color,
        );
    }
}

fn draw_shockwave(shockwave: &Shockwave, segments: usize) {
    let opacity = shockwave.opacity();
    if opacity <= 0.001 {
        return;
    }
    draw_ring(
        shockwave.origin,
        shockwave.radius,
        segments,
        1.0,
        Color::new(0.90, 0.95, 1.0, opacity),
    );
}

fn draw_ring(origin: Vec2, radius: f32, segments: usize, thickness: f32, color: Color) {
    let segments = segments.max(12);
    let mut previous = origin + Vec2::new(radius, 0.0);
    for segment in 1..=segments {
        let angle = segment as f32 * std::f32::consts::TAU / segments as f32;
        let point = origin + Vec2::new(angle.cos(), angle.sin()) * radius;
        draw_line(previous.x, previous.y, point.x, point.y, thickness, color);
        previous = point;
    }
}

fn draw_ball(ball: &Ball, view: &ViewOptions) {
    let palette = palette(ball.material_kind);
    if view.trails {
        draw_trail(ball, palette.base);
    }

    draw_circle(
        ball.position.x + ball.radius * 0.28,
        ball.position.y + ball.radius * 0.38,
        ball.radius * 1.03,
        palette.shadow,
    );

    if ball.heat > 0.0 {
        draw_heat_glow(ball, palette);
    }

    draw_circle(
        ball.position.x,
        ball.position.y,
        ball.radius + 2.2,
        palette.rim,
    );
    draw_circle(ball.position.x, ball.position.y, ball.radius, palette.base);

    for i in 0..3 {
        let t = i as f32 / 3.0;
        let radius = ball.radius * (0.78 - t * 0.18);
        let offset = Vec2::new(
            -ball.radius * (0.20 + t * 0.06),
            -ball.radius * (0.24 + t * 0.04),
        );
        let color = mix_color(palette.base, palette.highlight, 0.16 + t * 0.16);
        draw_circle(
            ball.position.x + offset.x,
            ball.position.y + offset.y,
            radius,
            color,
        );
    }

    draw_material_mark(ball, palette);
    draw_spin_mark(ball);
}

fn draw_trail(ball: &Ball, base: Color) {
    if ball.trail.len() < 2 {
        return;
    }

    let len = ball.trail.len() as f32;
    let mut previous: Option<Vec2> = None;
    for (index, &point) in ball.trail.iter().enumerate() {
        if let Some(start) = previous {
            let age = index as f32 / len;
            let width = ball.radius * (0.18 + age * 0.34);
            let mut color = base;
            color.a = age.powf(1.8) * 0.28;
            draw_line(start.x, start.y, point.x, point.y, width, color);
            draw_circle(point.x, point.y, width * 0.5, color);
        }
        previous = Some(point);
    }
}

fn draw_heat_glow(ball: &Ball, palette: Palette) {
    let heat = ball.heat.clamp(0.0, 1.0);
    let glow = mix_color(palette.base, Color::new(1.0, 0.68, 0.20, 1.0), 0.25);
    let glow = Color::new(glow.r, glow.g, glow.b, 0.11 * heat);
    draw_circle(
        ball.position.x,
        ball.position.y,
        ball.radius * (1.22 + heat * 0.55),
        glow,
    );
    draw_circle_lines(
        ball.position.x,
        ball.position.y,
        ball.radius * (1.10 + heat * 0.28),
        1.4,
        Color::new(glow.r, glow.g, glow.b, 0.34 * heat),
    );
}

fn draw_material_mark(ball: &Ball, palette: Palette) {
    match ball.material_kind {
        MaterialKind::Rubber => {
            draw_circle_lines(
                ball.position.x,
                ball.position.y,
                ball.radius * 0.72,
                2.0,
                Color::new(0.26, 0.04, 0.03, 0.55),
            );
        }
        MaterialKind::Steel => {
            let start = ball.position + Vec2::new(-ball.radius * 0.38, -ball.radius * 0.28);
            let end = ball.position + Vec2::new(ball.radius * 0.45, -ball.radius * 0.50);
            draw_line(
                start.x,
                start.y,
                end.x,
                end.y,
                2.2,
                Color::new(1.0, 1.0, 1.0, 0.82),
            );
        }
        MaterialKind::Glass => {
            draw_circle_lines(
                ball.position.x,
                ball.position.y,
                ball.radius * 0.68,
                1.6,
                palette.highlight,
            );
            draw_circle(
                ball.position.x + ball.radius * 0.35,
                ball.position.y - ball.radius * 0.30,
                ball.radius * 0.16,
                Color::new(1.0, 1.0, 1.0, 0.68),
            );
        }
    }
}

fn draw_spin_mark(ball: &Ball) {
    if ball.angular_velocity.abs() < 0.18 {
        return;
    }
    let direction = Vec2::new(ball.rotation.cos(), ball.rotation.sin());
    let end = ball.position + direction * ball.radius * 0.92;
    draw_line(
        ball.position.x,
        ball.position.y,
        end.x,
        end.y,
        1.4,
        Color::new(0.03, 0.04, 0.05, 0.62),
    );
}

fn draw_velocity_vectors(balls: &[Ball]) {
    for ball in balls {
        let start = ball.position;
        let velocity = ball.velocity.clamp_length_max(620.0) * 0.12;
        let end = start + velocity;
        draw_line(
            start.x,
            start.y,
            end.x,
            end.y,
            1.4,
            Color::new(0.35, 1.0, 0.62, 0.78),
        );
        let dir = velocity.normalize_or_zero();
        if dir.length_squared() > 0.0 {
            let side = Vec2::new(-dir.y, dir.x);
            draw_triangle(
                end,
                end - dir * 8.0 + side * 4.5,
                end - dir * 8.0 - side * 4.5,
                Color::new(0.35, 1.0, 0.62, 0.78),
            );
        }
    }
}

fn draw_contact_normals(normals: &[ContactNormal]) {
    for normal in normals {
        let alpha = normal.alpha();
        let end = normal.origin + normal.normal * normal.length;
        draw_line(
            normal.origin.x,
            normal.origin.y,
            end.x,
            end.y,
            2.0,
            Color::new(1.0, 0.82, 0.25, 0.78 * alpha),
        );
    }
}

fn draw_compact_overlay(world: &World, overlay: OverlayState) {
    let counts = world.material_counts();
    let paused = if overlay.paused { " | Paused" } else { "" };

    draw_rectangle(14.0, 14.0, 620.0, 96.0, PANEL);
    draw_text_ex(
        &format!(
            "FPS {} | Physics {}Hz x{} | Steps {} | Coll/frame {}{}",
            overlay.fps,
            overlay.fixed_hz,
            overlay.substeps,
            overlay.last_steps,
            world.stats.frame_collisions,
            paused
        ),
        26.0,
        40.0,
        text_params(20, TEXT),
    );
    draw_text_ex(
        &format!(
            "Kinetic {:.0} | Potential {:.0} | Driven dE {:+.1}% | Speed {:.2}x",
            world.stats.kinetic_energy,
            world.stats.potential_energy,
            world.stats.energy_drift * 100.0,
            overlay.time_scale
        ),
        26.0,
        65.0,
        text_params(18, TEXT),
    );
    draw_text_ex(
        &format!(
            "Balls {} | Rubber {}  Steel {}  Glass {} | Seed {}",
            world.balls.len(),
            counts.rubber,
            counts.steel,
            counts.glass,
            world.seed
        ),
        26.0,
        90.0,
        text_params(18, MUTED_TEXT),
    );

    if overlay.dropped_time > 0.0 {
        draw_text_ex(
            &format!("Dropped catch-up: {:.1}ms", overlay.dropped_time * 1000.0),
            590.0,
            40.0,
            text_params(17, WARNING),
        );
    }
}

fn draw_help_overlay(world: &World) {
    let x = 14.0;
    let y = 122.0;
    draw_rectangle(x, y, 492.0, 214.0, PANEL);
    draw_text_ex("Controls", x + 14.0, y + 30.0, text_params(20, ACCENT));
    draw_text_ex(
        "P pause   . step   R reset   1/2/3 speed   Tab HUD",
        x + 14.0,
        y + 58.0,
        text_params(17, TEXT),
    );
    draw_text_ex(
        "V velocity   N normals   T trails   E effects   H help",
        x + 14.0,
        y + 82.0,
        text_params(17, TEXT),
    );

    draw_text_ex("Materials", x + 14.0, y + 120.0, text_params(20, ACCENT));
    let mut row_y = y + 148.0;
    for kind in MaterialKind::ALL {
        let material = kind.material();
        let palette = palette(kind);
        draw_circle(x + 24.0, row_y - 5.0, 6.0, palette.base);
        draw_text_ex(
            &format!(
                "{}  e={:.2}  mu={:.2}  density={:.1}",
                material.name, material.restitution, material.friction, material.density
            ),
            x + 42.0,
            row_y,
            text_params(17, TEXT),
        );
        row_y += 24.0;
    }

    draw_text_ex(
        &format!(
            "Effects capped: ripples {}  shockwaves {}",
            world.config.effects.max_ripples, world.config.effects.max_shockwaves
        ),
        x + 14.0,
        y + 202.0,
        text_params(16, MUTED_TEXT),
    );
}

fn draw_hover_inspector(world: &World, viewport: Viewport) {
    let Some(world_mouse) = mouse_to_world(viewport, &world.config.render) else {
        return;
    };
    let Some(ball) = world.nearest_ball(world_mouse, 34.0) else {
        return;
    };

    let material = ball.material();
    let screen = world_to_screen(
        ball.position + Vec2::new(ball.radius + 10.0, -ball.radius - 10.0),
        viewport,
    );
    let panel_w = 202.0;
    let panel_h = 94.0;
    let x = screen.x.min(screen_width() - panel_w - 12.0).max(12.0);
    let y = screen.y.min(screen_height() - panel_h - 12.0).max(12.0);
    draw_rectangle(x, y, panel_w, panel_h, PANEL);
    draw_text_ex(
        material.name,
        x + 12.0,
        y + 24.0,
        text_params(19, palette(ball.material_kind).base),
    );
    draw_text_ex(
        &format!("speed {:>5.0} px/s", ball.velocity.length()),
        x + 12.0,
        y + 48.0,
        text_params(16, TEXT),
    );
    draw_text_ex(
        &format!("mass {:.2}  spin {:.2}", ball.mass, ball.angular_velocity),
        x + 12.0,
        y + 70.0,
        text_params(16, MUTED_TEXT),
    );
    draw_text_ex(
        &format!("heat {:.0}%", ball.heat * 100.0),
        x + 12.0,
        y + 90.0,
        text_params(16, MUTED_TEXT),
    );
}

fn draw_control_hint() {
    let text = "H help  P pause  R reset";
    let width = measure_text(text, None, 16, 1.0).width;
    draw_text_ex(
        text,
        screen_width() - width - 16.0,
        screen_height() - 18.0,
        text_params(16, MUTED_TEXT),
    );
}

fn mouse_to_world(viewport: Viewport, config: &RenderConfig) -> Option<Vec2> {
    let (mx, my) = mouse_position();
    if mx < viewport.x
        || my < viewport.y
        || mx > viewport.x + viewport.width
        || my > viewport.y + viewport.height
    {
        return None;
    }

    Some(
        Vec2::new(
            (mx - viewport.x) / viewport.scale,
            (my - viewport.y) / viewport.scale,
        )
        .clamp(Vec2::ZERO, config.world_size()),
    )
}

fn world_to_screen(world: Vec2, viewport: Viewport) -> Vec2 {
    Vec2::new(
        viewport.x + world.x * viewport.scale,
        viewport.y + world.y * viewport.scale,
    )
}

fn palette(kind: MaterialKind) -> Palette {
    match kind {
        MaterialKind::Rubber => Palette {
            base: Color::new(0.94, 0.22, 0.14, 1.0),
            rim: Color::new(0.35, 0.055, 0.045, 1.0),
            highlight: Color::new(1.0, 0.50, 0.32, 0.92),
            shadow: Color::new(0.02, 0.01, 0.01, 0.34),
        },
        MaterialKind::Steel => Palette {
            base: Color::new(0.70, 0.76, 0.86, 1.0),
            rim: Color::new(0.26, 0.31, 0.38, 1.0),
            highlight: Color::new(0.96, 0.99, 1.0, 0.95),
            shadow: Color::new(0.01, 0.015, 0.02, 0.36),
        },
        MaterialKind::Glass => Palette {
            base: Color::new(0.19, 0.75, 0.96, 0.94),
            rim: Color::new(0.05, 0.30, 0.42, 1.0),
            highlight: Color::new(0.82, 1.0, 1.0, 0.72),
            shadow: Color::new(0.00, 0.025, 0.035, 0.30),
        },
    }
}

fn color_from_rgb(rgb: Rgb, alpha: f32) -> Color {
    Color::new(rgb.r, rgb.g, rgb.b, alpha)
}

fn mix_color(a: Color, b: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * amount,
        a.g + (b.g - a.g) * amount,
        a.b + (b.b - a.b) * amount,
        a.a + (b.a - a.a) * amount,
    )
}

fn text_params(font_size: u16, color: Color) -> TextParams<'static> {
    TextParams {
        font_size,
        color,
        ..Default::default()
    }
}
