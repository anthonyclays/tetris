use std::f32::consts::FRAC_PI_2;

use na::{Vector1, Rotation2, Point2, OrthographicMatrix3};
use ncollide::shape;

use glium::Surface;
use glium::backend::Facade;
use glium::draw_parameters::DrawParameters;
use glium::index::{IndexBuffer, PrimitiveType};
use glium::program::Program;
use glium::vertex::VertexBuffer;
use glium_text::{TextSystem, FontTexture, TextDisplay};

use consts::*;
use game::*;

pub struct GraphicsProperties<'a> {
    proj: [[f32; 4]; 4],
    program: Program,
    draw_params: DrawParameters<'a>,
    text_system: TextSystem,
    font: FontTexture,
    text_proj: [[f32; 4]; 4],
}

impl<'a> GraphicsProperties<'a> {
    pub fn new<F: Facade>(display: &F) -> Self {
        GraphicsProperties {
            proj: *OrthographicMatrix3::new(LEFT, RIGHT, BOTTOM, TOP, -1.0, 1.0).as_matrix().as_ref(),
            program: Program::from_source(display,
                                          &include_str!("../res/shaders/tetris.vs"),
                                          &include_str!("../res/shaders/tetris.fs"),
                                          None).unwrap(),
            draw_params: DrawParameters {
                point_size: Some(10.0),
                ..Default::default()
            },
            text_system: TextSystem::new(display),
            font: FontTexture::new(display, &include_bytes!("../res/fonts/Roboto-Regular.ttf")[..], 100).unwrap(),
            text_proj: *OrthographicMatrix3::new(-1.0, 23.0, -30.0, 2.0, -1.0, 1.0).as_matrix().as_ref(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

impl Game {
    pub fn draw<S: Surface, F: Facade>(&self, display: &F, target: &mut S, props: &GraphicsProperties) {
        // Draw the blocks
        let mut vertices = vec![];
        let mut indices: Vec<u32> = vec![];
        let mut n = 0;

        for Tetromino { rbh, color } in self.tetrominos().cloned() {
            let rb = rbh.borrow();
            let &iso = rb.position();
            let inner_shapes = rb.shape().as_shape::<shape::Compound<_, _>>().unwrap().shapes();

            for &(inner_iso, _) in inner_shapes.iter() {
                let transform = iso * inner_iso;
                const L1: f32 = BLOCK_SIZE / 2.0 - CORNER_RADIUS;
                let rel_pts: Vec<Point2<_>> = (0..(EDGES_PER_CORNER+1)).map(|n| {
                    let angle = n as f32 * FRAC_PI_2 / EDGES_PER_CORNER as f32;
                    Point2::new(L1 + CORNER_RADIUS * angle.cos(), L1 + CORNER_RADIUS * angle.sin())
                }).collect();
                vertices.extend(rel_pts.iter().map(|&pt| {
                    let pos = transform * pt;
                    Vertex { position: [pos.x, pos.y], color: color }
                }));
                vertices.extend(rel_pts.iter().map(|&pt| {
                    let pos = transform * Rotation2::new(Vector1::new(FRAC_PI_2)) * pt;
                    Vertex { position: [pos.x, pos.y], color: color }
                }));
                vertices.extend(rel_pts.iter().map(|&pt| {
                    let pos = transform * Rotation2::new(Vector1::new(2.0 * FRAC_PI_2)) * pt;
                    Vertex { position: [pos.x, pos.y], color: color }
                }));
                vertices.extend(rel_pts.iter().map(|&pt| {
                    let pos = transform * Rotation2::new(Vector1::new(3.0 * FRAC_PI_2)) * pt;
                    Vertex { position: [pos.x, pos.y], color: color }
                }));
                for i in 1..(VERTS_PER_BLOCK - 1) {
                    indices.extend_from_slice(&[n, n + i, n + i + 1]);
                }
                n += VERTS_PER_BLOCK;
            }
        }

        let uniforms = uniform! { proj: props.proj };
        let vb = VertexBuffer::new(display, &vertices[..]).unwrap();
        let ib = IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices[..]).unwrap();
        target.draw(&vb, &ib, &props.program, &uniforms, &props.draw_params).unwrap();

        // Draw text
        let text = TextDisplay::new(&props.text_system, &props.font, &format!("Score: {}", self.score()));
        ::glium_text::draw(&text, &props.text_system, target, props.text_proj, (1.0, 1.0, 1.0, 1.0));
    }
}

pub fn show_loading_screen<F: Facade, S: Surface>(_display: &F, target: &mut S) {
    target.clear_color(1.0, 1.0, 1.0, 1.0);
}
