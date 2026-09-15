//! Renders one frame offscreen and checks the pixels. Skips (with a message)
//! when no GPU adapter is available; Mesa's lavapipe is enough.
//!
//! The frame is also written to `$CARGO_TARGET_TMPDIR/screenshot.png`.

use std::fs::File;
use std::io::BufWriter;

use frustration_tetris::consts::{BLOCKS_PER_LINE, BOTTOM, LEFT, POLYOMINOS, SPAWN_POINT, TOP};
use frustration_tetris::render::Painter;
use frustration_tetris::{Game, view};

const WIDTH: u32 = 300;
const HEIGHT: u32 = 400;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn world_to_pixel(x: f32, y: f32) -> (u32, u32) {
    let m = view::playfield_projection(WIDTH, HEIGHT);
    let cx = m[0][0] * x + m[3][0];
    let cy = m[1][1] * y + m[3][1];
    (
        ((cx + 1.0) / 2.0 * WIDTH as f32) as u32,
        ((1.0 - cy) / 2.0 * HEIGHT as f32) as u32,
    )
}

#[test]
fn renders_blocks_and_score_offscreen() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        eprintln!("no GPU adapter available, skipping");
        return;
    };
    eprintln!("using adapter: {:?}", adapter.get_info().name);
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("device");

    let mut game = Game::new();
    game.spawn(&POLYOMINOS[1], 0.0, [1.0, 0.1, 0.1]);
    for _ in 0..200 {
        game.step();
    }

    let mut painter = Painter::new(device.clone(), queue.clone(), FORMAT, WIDTH, HEIGHT);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    painter.paint(&game, &target.create_view(&Default::default()));

    let pixels = read_back(&device, &queue, &target);
    let pixel = |(x, y): (u32, u32)| {
        let i = 4 * (y * WIDTH + x) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    };

    // The piece fell from the spawn point and now rests on the floor, in
    // the middle of the field.
    let block = game.blocks().next().expect("piece exists");
    assert!(
        block.position[1] < SPAWN_POINT.y - 1.0,
        "piece should have fallen"
    );
    let [r, g, b] = pixel(world_to_pixel(block.position[0], block.position[1]));
    assert!(
        r > 200 && g < 100 && b < 100,
        "expected a red block, got {r},{g},{b}"
    );
    // The spawn point is empty again.
    assert_eq!(
        pixel(world_to_pixel(SPAWN_POINT.x, SPAWN_POINT.y)),
        [0, 0, 0]
    );
    // The bottom-left corner is empty.
    let corner = world_to_pixel(LEFT + 0.3, BOTTOM + 0.3);
    assert_eq!(pixel(corner), [0, 0, 0]);
    // The score label draws something white-ish near the top-left.
    let [ax, ay] = view::SCORE_ANCHOR;
    let (x0, y0) = world_to_pixel(ax, ay);
    let (x1, y1) = world_to_pixel(ax + 3.0, ay - view::SCORE_HEIGHT);
    let bright = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&p| pixel(p).iter().all(|&c| c > 150))
        .count();
    assert!(bright > 20, "expected score text pixels, found {bright}");
    // Nothing is drawn outside the field: a full row's width fits.
    assert!(world_to_pixel(LEFT + BLOCKS_PER_LINE as f32, TOP).0 <= WIDTH);

    let dir = env!("CARGO_TARGET_TMPDIR");
    std::fs::create_dir_all(dir).unwrap();
    let path = format!("{dir}/screenshot.png");
    let mut encoder =
        png::Encoder::new(BufWriter::new(File::create(&path).unwrap()), WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    eprintln!("wrote {path}");
}

fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_row = (4 * WIDTH).div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    buffer.map_async(wgpu::MapMode::Read, .., |r| r.expect("map readback buffer"));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll");
    let data = buffer.get_mapped_range(..).expect("mapped range");
    data.chunks(padded_row as usize)
        .flat_map(|row| &row[..(4 * WIDTH) as usize])
        .copied()
        .collect()
}
