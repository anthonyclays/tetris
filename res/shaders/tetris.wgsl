// Blocks: one shared rounded-square outline, instanced per block.

struct Globals {
    proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;

struct BlockVertex {
    @location(0) position: vec2<f32>,
};

struct BlockInstance {
    @location(1) position: vec2<f32>,
    @location(2) angle: f32,
    @location(3) color: vec3<f32>,
};

struct BlockOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_block(v: BlockVertex, i: BlockInstance) -> BlockOutput {
    let c = cos(i.angle);
    let s = sin(i.angle);
    let p = vec2<f32>(c * v.position.x - s * v.position.y, s * v.position.x + c * v.position.y) + i.position;
    var out: BlockOutput;
    out.clip = globals.proj * vec4<f32>(p, 0.0, 1.0);
    out.color = i.color;
    return out;
}

@fragment
fn fs_block(in: BlockOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

// Score label: a textured quad.

@group(1) @binding(0) var text_texture: texture_2d<f32>;
@group(1) @binding(1) var text_sampler: sampler;

struct TextVertex {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct TextOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_text(v: TextVertex) -> TextOutput {
    var out: TextOutput;
    out.clip = globals.proj * vec4<f32>(v.position, 0.0, 1.0);
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_text(in: TextOutput) -> @location(0) vec4<f32> {
    return textureSample(text_texture, text_sampler, in.uv);
}
