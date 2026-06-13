#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterialKind {
    Rubber,
    Steel,
    Glass,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub kind: MaterialKind,
    pub name: &'static str,
    pub density: f32,
    pub restitution: f32,
    pub friction: f32,
    pub drag_coefficient: f32,
    pub display_rgb: Rgb,
}

#[derive(Clone, Copy, Debug)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn mix(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * amount,
            g: self.g + (other.g - self.g) * amount,
            b: self.b + (other.b - self.b) * amount,
        }
    }
}

impl MaterialKind {
    pub const ALL: [Self; 3] = [Self::Rubber, Self::Steel, Self::Glass];

    pub const fn material(self) -> Material {
        match self {
            Self::Rubber => Material {
                kind: Self::Rubber,
                name: "Rubber",
                density: 0.85,
                restitution: 0.88,
                friction: 0.82,
                drag_coefficient: 0.55,
                display_rgb: Rgb::new(0.95, 0.20, 0.13),
            },
            Self::Steel => Material {
                kind: Self::Steel,
                name: "Steel",
                density: 3.2,
                restitution: 0.58,
                friction: 0.34,
                drag_coefficient: 0.42,
                display_rgb: Rgb::new(0.70, 0.76, 0.86),
            },
            Self::Glass => Material {
                kind: Self::Glass,
                name: "Glass",
                density: 1.8,
                restitution: 0.94,
                friction: 0.18,
                drag_coefficient: 0.38,
                display_rgb: Rgb::new(0.24, 0.82, 1.0),
            },
        }
    }
}

pub const WALL_MATERIAL: Material = Material {
    kind: MaterialKind::Steel,
    name: "Wall",
    density: 1000.0,
    restitution: 0.72,
    friction: 0.48,
    drag_coefficient: 0.0,
    display_rgb: Rgb::new(0.78, 0.80, 0.92),
};

#[inline]
pub fn combine_restitution(a: Material, b: Material) -> f32 {
    a.restitution.min(b.restitution).clamp(0.0, 1.0)
}

#[inline]
pub fn combine_friction(a: Material, b: Material) -> f32 {
    (a.friction.max(0.0) * b.friction.max(0.0)).sqrt()
}
