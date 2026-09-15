#version 140

in vec2 v_uv;
uniform sampler2D tex;
out vec4 color;

void main() {
    color = texture(tex, v_uv);
}
