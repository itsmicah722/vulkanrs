/// This script runs compiles all GLSL shaders in the `shaders/` directory into SPIR-V bytecode
/// usable by the Vulkan graphics pipeline.
use std::{env, error::Error, fs, path::Path};

use shaderc::{CompileOptions, Compiler, OptimizationLevel, ResolvedInclude, ShaderKind};

/// Returns an error if any shader fails to compile or cannot be read from / written to.
fn main() -> Result<(), Box<dyn Error>> {
    compile_shaders()?;

    Ok(())
}

/// Compiles all `.glsl` files in `shaders/` to SPIR-V `.spv` in the Cargo `OUT_DIR`.
fn compile_shaders() -> Result<(), Box<dyn Error>> {
    // Create a ShaderC compiler instance.
    let compiler = Compiler::new().expect("Failed to initialize shader compiler");

    // Set up compile options: include paths, macros, optimization levels, etc.
    let mut options = CompileOptions::new().expect("Failed to create compile options");

    // Allow `#include "file.glsl"` directives to refer to files in `shaders/`.
    options.set_include_callback(|requested, _include_type, _source, _depth| {
        let include_path = Path::new("shaders").join(requested);

        let content = fs::read_to_string(&include_path)
            .map_err(|e| format!("Could not include '{requested}': {e}"))?;

        Ok(ResolvedInclude {
            resolved_name: include_path.to_string_lossy().into_owned(),
            content,
        })
    });

    // Choose optimization based on build profile.
    match env::var("PROFILE").as_deref() {
        Ok("release") => options.set_optimization_level(OptimizationLevel::Performance),
        _ => options.set_optimization_level(OptimizationLevel::Zero),
    }

    // Where to place compiled SPIR-V bytecode files.
    let out_dir = env::var("OUT_DIR")?;
    println!("cargo:rustc-env=SHADER_OUT_DIR={out_dir}");

    // Scan the `shaders/` directory for `.glsl` files
    for entry in fs::read_dir("shaders")? {
        let entry = entry?;
        let path = entry.path();

        // Only process files ending in `.glsl`
        if path.extension().and_then(|s| s.to_str()) != Some("glsl") {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy();

        let kind = if filename.ends_with(".vert.glsl") {
            ShaderKind::Vertex
        } else if filename.ends_with(".frag.glsl") {
            ShaderKind::Fragment
        } else if filename.ends_with(".comp.glsl") {
            ShaderKind::Compute
        } else if filename.ends_with(".geom.glsl") {
            ShaderKind::Geometry
        } else if filename.ends_with(".tesc.glsl") {
            ShaderKind::TessControl
        } else if filename.ends_with(".tese.glsl") {
            ShaderKind::TessEvaluation
        } else {
            panic!(
                "Unrecognized shader type for file '{filename}'. Use a suffix like .vert.glsl or .frag\
                .glsl"
            );
        };

        // Read the GLSL source code to string.
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read shader '{filename}': {e}"));

        // Compile GLSL text to SPIR-V bytecode.
        let artifact =
            compiler.compile_into_spirv(&source, kind, &filename, "main", Some(&options))?;

        // Write out the `.spv` file with the same base name.
        let spv_name = filename.replace(".glsl", ".spv");
        let dest_path = Path::new(&out_dir).join(&spv_name);
        fs::write(&dest_path, artifact.as_binary_u8())?;

        // Re-run if this specific shader changes.
        println!("cargo:rerun-if-changed={}", path.display());
    }

    // Watch the all files in `shaders/` directory for changes, and rerun if changed.
    println!("cargo:rerun-if-changed=shaders");

    Ok(())
}
