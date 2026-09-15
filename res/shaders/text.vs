#version 140

in vec2 position;
in vec2 uv;

uniform mat4 proj;

out vec2 v_uv;

void main() {
    gl_Position = proj * vec4(position, 0.0, 1.0);
    v_uv = uv;
}
