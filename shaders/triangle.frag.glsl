// Tells the Vulkan driver we're using GLSL that targets Vulkan's 1.2+
// core specification (GLSL 4.50)
#version 450

// This input comes from the vertex shader's output: `fragColor`. They do not have
// the same name because they use the same index: `0`.
layout(location = 0) in vec3 fragColor;

// This is the final output that the fragment shader writes into the current render
// target (Vulkan swapchain image's color attachment).
layout(location = 0) out vec4 outColor;

void main() {
    // We convert fragColor (vec3)  into outColor (vec4), by adding the alpha channel
    // 1.0 and hand that complete RGBA value to be stored on the screen.
    outColor = vec4(fragColor, 1.0);
}