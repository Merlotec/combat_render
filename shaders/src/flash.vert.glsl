#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(std140, set = 0, binding = 0) uniform ViewArgs {
    uniform mat4 proj;
    uniform mat4 view;
    uniform mat4 proj_view;
};

const uint MAX_FLASHES = 10;
struct FlashData {
    vec3 center;
    float scale;
};

layout(std140, set = 1, binding = 0) uniform Stars {
    uint flash_count;
    FlashData[MAX_FLASHES] flashes;
};

layout(location = 0) in vec3 pos;
layout(location = 1) in vec2 uv;

layout(location = 0) flat out uint idx;
layout(location = 1) out vec2 _uv;

void main() {
    FlashData flash = flashes[gl_InstanceIndex];
    vec4 c_worldspace = view * vec4(flash.center, 1);
    vec3 scaled_offset = pos * flash.scale;
    vec3 cameraspace = c_worldspace.xyz + pos;
    vec4 screenspace = proj * vec4(cameraspace, 1);
    idx = gl_InstanceIndex;
    _uv = uv;
    gl_Position = screenspace;
}