//! OpenGL rendering with glium.
//!
//! Every block is drawn as an instance of one static rounded-square mesh:
//! the outline lives in a vertex buffer uploaded once, and each frame only a
//! small per-instance buffer (position, angle, colour per block) is
//! rewritten. The score is rasterised on the CPU when it changes and drawn
//! as a single textured quad.

use ab_glyph::FontRef;
use frustration_tetris::text::{self, TextImage};
use frustration_tetris::{BlockInstance, Game, mesh, view};
use glium::glutin::surface::WindowSurface;
use glium::index::{IndexBuffer, NoIndices, PrimitiveType};
use glium::texture::{RawImage2d, Texture2d};
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
use glium::{
    Blend, Display, DrawParameters, Program, Surface, VertexBuffer, implement_vertex, uniform,
};

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
struct Instance {
    i_position: [f32; 2],
    i_angle: f32,
    i_color: [f32; 3],
}
implement_vertex!(Instance, i_position, i_angle, i_color);

impl From<BlockInstance> for Instance {
    fn from(b: BlockInstance) -> Self {
        Instance {
            i_position: b.position,
            i_angle: b.angle,
            i_color: b.color,
        }
    }
}

#[derive(Copy, Clone)]
struct TextVertex {
    position: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(TextVertex, position, uv);

const TEXT_PX: f32 = 64.0;

struct ScoreLabel {
    value: usize,
    texture: Texture2d,
    quad: VertexBuffer<TextVertex>,
}

pub struct Renderer {
    proj: view::Mat4,
    block_program: Program,
    block_vertices: VertexBuffer<Vertex>,
    block_indices: IndexBuffer<u16>,
    instances: VertexBuffer<Instance>,
    text_program: Program,
    font: FontRef<'static>,
    score: Option<ScoreLabel>,
}

impl Renderer {
    pub fn new(display: &Display<WindowSurface>) -> Self {
        let (width, height) = display.get_framebuffer_dimensions();
        let vertices: Vec<Vertex> = mesh::block_outline()
            .into_iter()
            .map(|position| Vertex { position })
            .collect();
        Renderer {
            proj: view::playfield_projection(width, height),
            block_program: Program::from_source(
                display,
                include_str!("../res/shaders/tetris.vs"),
                include_str!("../res/shaders/tetris.fs"),
                None,
            )
            .expect("block shader compiles"),
            block_vertices: VertexBuffer::immutable(display, &vertices).expect("vertex buffer"),
            block_indices: IndexBuffer::immutable(
                display,
                PrimitiveType::TrianglesList,
                &mesh::block_indices(),
            )
            .expect("index buffer"),
            instances: VertexBuffer::empty_dynamic(display, 256).expect("instance buffer"),
            text_program: Program::from_source(
                display,
                include_str!("../res/shaders/text.vs"),
                include_str!("../res/shaders/text.fs"),
                None,
            )
            .expect("text shader compiles"),
            font: text::default_font(),
            score: None,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.proj = view::playfield_projection(width, height);
    }

    pub fn draw(&mut self, display: &Display<WindowSurface>, game: &Game) {
        let instances: Vec<Instance> = game.blocks().map(Into::into).collect();
        if instances.len() > self.instances.len() {
            self.instances =
                VertexBuffer::empty_dynamic(display, instances.len() * 2).expect("instance buffer");
        }
        let used = self.instances.slice(0..instances.len()).expect("in range");
        used.write(&instances);

        if self.score.as_ref().map(|s| s.value) != Some(game.score()) {
            self.score = Some(self.score_label(display, game.score()));
        }

        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        if !instances.is_empty() {
            frame
                .draw(
                    (
                        &self.block_vertices,
                        used.per_instance().expect("instancing supported"),
                    ),
                    &self.block_indices,
                    &self.block_program,
                    &uniform! { proj: self.proj },
                    &DrawParameters::default(),
                )
                .expect("draw blocks");
        }
        if let Some(score) = &self.score {
            let sampler = score
                .texture
                .sampled()
                .magnify_filter(MagnifySamplerFilter::Linear)
                .minify_filter(MinifySamplerFilter::Linear);
            frame
                .draw(
                    &score.quad,
                    NoIndices(PrimitiveType::TriangleStrip),
                    &self.text_program,
                    &uniform! { proj: self.proj, tex: sampler },
                    &DrawParameters {
                        blend: Blend::alpha_blending(),
                        ..DrawParameters::default()
                    },
                )
                .expect("draw score");
        }
        frame.finish().expect("swap buffers");
    }

    fn score_label(&self, display: &Display<WindowSurface>, value: usize) -> ScoreLabel {
        let image = text::rasterize(&self.font, &format!("Score: {value}"), TEXT_PX);
        let quad: Vec<TextVertex> = view::text_quad(&image, view::SCORE_ANCHOR, view::SCORE_HEIGHT)
            .into_iter()
            .map(|[x, y, u, v]| TextVertex {
                position: [x, y],
                uv: [u, v],
            })
            .collect();
        ScoreLabel {
            value,
            texture: upload(display, &image),
            quad: VertexBuffer::immutable(display, &quad).expect("text quad"),
        }
    }
}

/// Upload a bitmap with rows stored top to bottom; `v = 0` then samples the
/// top row, matching [`view::text_quad`].
fn upload(display: &Display<WindowSurface>, image: &TextImage) -> Texture2d {
    let raw = RawImage2d::from_raw_rgba(image.rgba.clone(), (image.width, image.height));
    Texture2d::new(display, raw).expect("text texture")
}
