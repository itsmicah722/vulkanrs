// Tells the Vulkan driver we're using GLSL that targets Vulkan's 1.2+
// core specification (GLSL 4.50)
#version 450

// Declare an output color to the fragment shader at location 0
layout(location = 0) out vec3 fragColor;

// Contains the XY positions for each vertex
vec2 positions[3] = vec2[](
    vec2(0.0, -0.5),    // Top center
    vec2(0.5, 0.5),     // Bottom right
    vec2(-0.5, 0.5)     // Bottom left
);

// Contians the RGB colors for each vertex
vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),    // Blue
    vec3(0.0, 1.0, 0.0),    // Green
    vec3(0.0, 0.0, 1.0)     // Red
);

/*
     - `gl_VertexIndex` is a built-in variable that tells the vertex shader which
     vertex it's currently processing. This index is provided by the GPU when a
     Vulkan draw command like vkCmdDraw() is called.

     - `gl_Position` is a built-in vec4 variable that the vertex shader MUST write
     to. It tells the GPU where to draw each vertex on the screen after all math and
     transformations operations are done.

     - `main()` runs once per vertex in the vertex shader concurrently on the GPU. This
     makes the shader *extremely* fast. Each main() call here is only different based on the
     current vertex being executed.
*/

void main() {
    // Set gl_Position to the current vertex in `positions`. The z coordinate is 0.0
    // because we are rendering a 2D triangle. The w coordinate is 1.0 so perspective division
    // holds no affect.
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);

    // Set the fragColor ouptut to the fragment shader to a element in `colors`
    // based on the current vertex index. This will cause color interpolation so the entire
    // triangle can be different colors.
    fragColor = colors[gl_VertexIndex];
}