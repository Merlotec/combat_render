#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(set = 2, binding = 0) uniform sampler2D glow_tex;

layout(location = 0) flat in uint idx;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 target;

void main() {
    target = texture(glow_tex, uv);
}
