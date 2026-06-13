#[derive(Clone, Copy, Debug)]
pub struct ViewOptions {
    pub compact_hud: bool,
    pub help_overlay: bool,
    pub trails: bool,
    pub effects: bool,
    pub velocity_vectors: bool,
    pub collision_normals: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            compact_hud: true,
            help_overlay: false,
            trails: true,
            effects: true,
            velocity_vectors: false,
            collision_normals: false,
        }
    }
}
