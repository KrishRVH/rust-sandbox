#![forbid(unsafe_code)]

use bouncing_ball_dodecahedron::app::App;
use bouncing_ball_dodecahedron::config::Config;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Bouncing Ball Physics Lab".to_owned(),
        window_width: 1120,
        window_height: 720,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new(Config::default());

    loop {
        app.handle_input();
        app.update(get_frame_time());
        app.draw();
        next_frame().await;
    }
}
