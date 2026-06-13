use macroquad::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct CircleSegmentContact {
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}

#[inline]
pub fn rotate_point_sincos(point: Vec2, sin: f32, cos: f32) -> Vec2 {
    Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

pub fn closest_point_on_segment(point: Vec2, start: Vec2, end: Vec2) -> Vec2 {
    let segment = end - start;
    let segment_len_sq = segment.length_squared();
    if segment_len_sq <= f32::EPSILON {
        return start;
    }

    let t = ((point - start).dot(segment) / segment_len_sq).clamp(0.0, 1.0);
    start + segment * t
}

pub fn circle_segment_contact(
    center: Vec2,
    radius: f32,
    start: Vec2,
    end: Vec2,
) -> Option<CircleSegmentContact> {
    let point = closest_point_on_segment(center, start, end);
    let delta = center - point;
    let distance = delta.length();

    if distance > radius {
        return None;
    }

    let edge = end - start;
    let normal = if distance > 0.0001 {
        delta / distance
    } else {
        Vec2::new(-edge.y, edge.x).normalize_or_zero()
    };

    Some(CircleSegmentContact {
        point,
        normal,
        distance,
    })
}

pub fn point_inside_polygon(vertices: &[Vec2], active_edges: &[bool], point: Vec2) -> bool {
    assert_eq!(
        vertices.len(),
        active_edges.len(),
        "polygon vertices and active edge masks must have equal length"
    );

    let mut inside = false;

    for i in 0..vertices.len() {
        if !active_edges[i] {
            continue;
        }

        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];

        let crosses_y = (a.y > point.y) != (b.y > point.y);
        if !crosses_y {
            continue;
        }

        let x_at_y = (b.x - a.x) * (point.y - a.y) / (b.y - a.y + f32::EPSILON) + a.x;
        if point.x < x_at_y {
            inside = !inside;
        }
    }

    inside
}

#[inline]
pub fn angular_velocity_at(point: Vec2, center: Vec2, angular_velocity: f32) -> Vec2 {
    let radius = point - center;
    angular_velocity * Vec2::new(-radius.y, radius.x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_segment_contact_detects_hit_and_miss() {
        let hit = circle_segment_contact(Vec2::new(3.0, 2.0), 2.5, Vec2::ZERO, Vec2::new(6.0, 0.0))
            .expect("circle should touch segment");
        assert!((hit.point - Vec2::new(3.0, 0.0)).length() < 0.001);
        assert!((hit.normal - Vec2::Y).length() < 0.001);

        assert!(
            circle_segment_contact(Vec2::new(3.0, 4.0), 1.0, Vec2::ZERO, Vec2::new(6.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn point_inside_polygon_respects_active_edges() {
        let vertices = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let closed = [true, true, true, true];
        assert!(point_inside_polygon(
            &vertices,
            &closed,
            Vec2::new(5.0, 5.0)
        ));
        assert!(!point_inside_polygon(
            &vertices,
            &closed,
            Vec2::new(15.0, 5.0)
        ));

        let disabled = [false, false, false, false];
        assert!(!point_inside_polygon(
            &vertices,
            &disabled,
            Vec2::new(5.0, 5.0)
        ));
    }
}
