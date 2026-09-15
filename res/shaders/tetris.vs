#version 140

// Per-vertex: the shared block outline, centred on the origin.
in vec2 position;
// Per-instance: where this block is, how it is rotated and its colour.
in vec2 i_position;
in float i_angle;
in vec3 i_color;

uniform mat4 proj;

out vec3 v_color;

void main() {
    float c = cos(i_angle);
    float s = sin(i_angle);
    vec2 p = vec2(c * position.x - s * position.y, s * position.x + c * position.y) + i_position;
    gl_Position = proj * vec4(p, 0.0, 1.0);
    v_color = i_color;
}
