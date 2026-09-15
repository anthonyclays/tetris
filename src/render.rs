//! Rendering with wgpu, independent of any window.
//!
//! [`Painter`] draws a [`Game`] into any texture view of the format it was
//! created for, so the same code serves the on-screen swapchain and
//! offscreen rendering (see `tests/screenshot.rs`).
//!
//! Every block is an instance of one static rounded-square mesh: the outline
//! and its indices are uploaded once, and each frame only the per-instance
//! buffer (position, angle, colour per block) is rewritten and drawn with a
//! single instanced draw call into a 4x multisampled target. The score is
//! rasterised on the CPU when it changes and drawn as one textured quad.

use std::mem::size_of;

use ab_glyph::FontRef;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::text::{self, TextImage};
use crate::{BlockInstance, Game, mesh, view};

pub const MSAA_SAMPLES: u32 = 4;
const TEXT_PX: f32 = 64.0;
const INITIAL_INSTANCE_CAPACITY: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    position: [f32; 2],
    angle: f32,
    color: [f32; 3],
}

impl From<BlockInstance> for Instance {
    fn from(b: BlockInstance) -> Self {
        Instance {
            position: b.position,
            angle: b.angle,
            color: b.color,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TextVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    proj: view::Mat4,
}

const BLOCK_VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: size_of::<Vertex>() as u64,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: size_of::<Instance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32, 3 => Float32x3],
};

const TEXT_VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: size_of::<TextVertex>() as u64,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
};

struct ScoreLabel {
    value: usize,
    bind_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
}

/// Draws the game into textures of one format and size.
pub struct Painter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    msaa_view: wgpu::TextureView,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    block_pipeline: wgpu::RenderPipeline,
    block_vertices: wgpu::Buffer,
    block_indices: wgpu::Buffer,
    index_count: u32,
    instances: wgpu::Buffer,
    instance_capacity: usize,
    text_pipeline: wgpu::RenderPipeline,
    text_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    font: FontRef<'static>,
    score: Option<ScoreLabel>,
}

impl Painter {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../res/shaders/tetris.wgsl"));

        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals"),
            contents: bytemuck::bytes_of(&Globals {
                proj: view::playfield_projection(width, height),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });

        let text_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let block_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blocks"),
                bind_group_layouts: &[Some(&globals_layout)],
                ..Default::default()
            });
        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text"),
            bind_group_layouts: &[Some(&globals_layout), Some(&text_layout)],
            ..Default::default()
        });
        let block_pipeline = create_pipeline(
            &device,
            &block_pipeline_layout,
            &shader,
            ("vs_block", "fs_block"),
            &[Some(BLOCK_VERTEX_LAYOUT), Some(INSTANCE_LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            None,
            format,
        );
        let text_pipeline = create_pipeline(
            &device,
            &text_pipeline_layout,
            &shader,
            ("vs_text", "fs_text"),
            &[Some(TEXT_VERTEX_LAYOUT)],
            wgpu::PrimitiveTopology::TriangleStrip,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            format,
        );

        let vertices: Vec<Vertex> = mesh::block_outline()
            .into_iter()
            .map(|position| Vertex { position })
            .collect();
        let indices = mesh::block_indices();
        let block_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("block vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let block_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("block indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instances = create_instance_buffer(&device, INITIAL_INSTANCE_CAPACITY);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("text"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Painter {
            msaa_view: create_msaa_view(&device, format, width, height),
            device,
            queue,
            format,
            size: (width, height),
            globals,
            globals_bind_group,
            block_pipeline,
            block_vertices,
            block_indices,
            index_count: indices.len() as u32,
            instances,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            text_pipeline,
            text_layout,
            sampler,
            font: text::default_font(),
            score: None,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Change the size of the target textures that `paint` will be given.
    pub fn resize(&mut self, width: u32, height: u32) {
        if (width, height) == self.size || width == 0 || height == 0 {
            return;
        }
        self.size = (width, height);
        self.msaa_view = create_msaa_view(&self.device, self.format, width, height);
        let globals = Globals {
            proj: view::playfield_projection(width, height),
        };
        self.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
    }

    /// Draw one frame of `game` into `target`, which must have the painter's
    /// format and size, and submit it to the queue.
    pub fn paint(&mut self, game: &Game, target: &wgpu::TextureView) {
        let instances: Vec<Instance> = game.blocks().map(Into::into).collect();
        if instances.len() > self.instance_capacity {
            self.instance_capacity = instances.len() * 2;
            self.instances = create_instance_buffer(&self.device, self.instance_capacity);
        }
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instances, 0, bytemuck::cast_slice(&instances));
        }
        if self.score.as_ref().map(|s| s.value) != Some(game.score()) {
            self.score = Some(self.score_label(game.score()));
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        // Only the resolved image is needed.
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            if !instances.is_empty() {
                pass.set_pipeline(&self.block_pipeline);
                pass.set_vertex_buffer(0, self.block_vertices.slice(..));
                pass.set_vertex_buffer(1, self.instances.slice(..));
                pass.set_index_buffer(self.block_indices.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..self.index_count, 0, 0..instances.len() as u32);
            }
            if let Some(score) = &self.score {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(1, &score.bind_group, &[]);
                pass.set_vertex_buffer(0, score.vertices.slice(..));
                pass.draw(0..4, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
    }

    fn score_label(&self, value: usize) -> ScoreLabel {
        let image = text::rasterize(&self.font, &format!("Score: {value}"), TEXT_PX);
        let texture = upload(&self.device, &self.queue, &image);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("score"),
            layout: &self.text_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let quad: Vec<TextVertex> = view::text_quad(&image, view::SCORE_ANCHOR, view::SCORE_HEIGHT)
            .into_iter()
            .map(|[x, y, u, v]| TextVertex {
                position: [x, y],
                uv: [u, v],
            })
            .collect();
        let vertices = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("score quad"),
                contents: bytemuck::cast_slice(&quad),
                usage: wgpu::BufferUsages::VERTEX,
            });
        ScoreLabel {
            value,
            bind_group,
            vertices,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    (vs, fs): (&str, &str),
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    topology: wgpu::PrimitiveTopology,
    blend: Option<wgpu::BlendState>,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(vs),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs),
            compilation_options: Default::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLES,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("block instances"),
        size: (capacity * size_of::<Instance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_msaa_view(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: MSAA_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// Upload a bitmap with rows stored top to bottom; `v = 0` then samples the
/// top row, matching [`view::text_quad`].
fn upload(device: &wgpu::Device, queue: &wgpu::Queue, image: &TextImage) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width: image.width,
        height: image.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("text"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * image.width),
            rows_per_image: Some(image.height),
        },
        size,
    );
    texture
}
