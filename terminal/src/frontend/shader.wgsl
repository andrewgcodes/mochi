// Terminal renderer shader
// Renders textured quads with color modulation

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    return out;
}

@group(0) @binding(0)
var t_atlas: texture_2d<f32>;
@group(0) @binding(1)
var s_atlas: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the atlas texture
    let tex_color = textureSample(t_atlas, s_atlas, in.tex_coords);
    
    // If texture coordinates are zero (no texture), use solid color
    if (in.tex_coords.x == 0.0 && in.tex_coords.y == 0.0) {
        return in.color;
    }
    
    // Otherwise, modulate texture alpha with vertex color
    return vec4<f32>(in.color.rgb, in.color.a * tex_color.a);
}
