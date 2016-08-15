#version 140

in vec2 position;
in vec3 color;
out vec3 f_color;

uniform mat4 proj;

void main() {
    gl_Position = proj * vec4(position, 0.0, 1.0);
    f_color = color;
}
