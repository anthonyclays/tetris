//! macroquad frontend: the whole window, input, drawing and text stack in one
//! dependency. Blocks are batched by macroquad into a handful of draw calls
//! per frame rather than GPU-instanced, which is plenty for a few hundred
//! blocks and keeps this frontend under a hundred lines.

use std::time::Duration;

use frustration_tetris::consts::{BOTTOM, LEFT, RIGHT, TOP};
use frustration_tetris::{Game, Key, controls, mesh};
use macroquad::prelude::*;

const KEYS: [(KeyCode, Key); 10] = [
    (KeyCode::Up, Key::Up),
    (KeyCode::Down, Key::Down),
    (KeyCode::Left, Key::Left),
    (KeyCode::Right, Key::Right),
    (KeyCode::H, Key::H),
    (KeyCode::J, Key::J),
    (KeyCode::K, Key::K),
    (KeyCode::L, Key::L),
    (KeyCode::Space, Key::Space),
    (KeyCode::Backspace, Key::Backspace),
];

fn window_conf() -> Conf {
    Conf {
        window_title: "Frustration Tetris".to_owned(),
        window_width: 600,
        window_height: 800,
        sample_count: 4,
        ..Default::default()
    }
}

/// Camera showing the whole playfield, centred and letterboxed so blocks
/// stay square whatever the window size.
fn playfield_camera() -> Camera2D {
    let (field_w, field_h) = (RIGHT - LEFT, TOP - BOTTOM);
    let aspect = screen_width() / screen_height();
    let (view_w, view_h) = if aspect > field_w / field_h {
        (field_h * aspect, field_h)
    } else {
        (field_w, field_w / aspect)
    };
    Camera2D {
        target: vec2((LEFT + RIGHT) / 2.0, (BOTTOM + TOP) / 2.0),
        zoom: vec2(2.0 / view_w, 2.0 / view_h),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();
    let outline = mesh::block_outline();
    let indices = mesh::block_indices();
    let font = load_ttf_font_from_bytes(include_bytes!("../res/fonts/Roboto-Regular.ttf"))
        .expect("bundled font loads");

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        for (code, key) in KEYS {
            if is_key_pressed(code)
                && let Some(action) = controls::on_press(key)
            {
                game.execute(action);
            }
            if is_key_released(code)
                && let Some(action) = controls::on_release(key)
            {
                game.execute(action);
            }
        }
        game.advance(Duration::from_secs_f32(get_frame_time()));

        clear_background(BLACK);
        set_camera(&playfield_camera());
        for block in game.blocks() {
            let (s, c) = block.angle.sin_cos();
            let [x, y] = block.position;
            let [r, g, b] = block.color;
            let color = Color::new(r, g, b, 1.0);
            let vertices = outline
                .iter()
                .map(|&[px, py]| {
                    Vertex::new(
                        c * px - s * py + x,
                        s * px + c * py + y,
                        0.0,
                        0.0,
                        0.0,
                        color,
                    )
                })
                .collect();
            draw_mesh(&Mesh {
                vertices,
                indices: indices.clone(),
                texture: None,
            });
        }

        set_default_camera();
        draw_text_ex(
            format!("Score: {}", game.score()),
            20.0,
            48.0,
            TextParams {
                font: Some(&font),
                font_size: 36,
                color: WHITE,
                ..Default::default()
            },
        );
        next_frame().await;
    }
}
