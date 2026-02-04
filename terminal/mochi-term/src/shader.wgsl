// Terminal rendering shader
// Renders textured quads with foreground and background colors

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bg_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) bg_color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    out.bg_color = in.bg_color;
    return out;
}

@group(0) @binding(0)
var glyph_texture: texture_2d<f32>;
@group(0) @binding(1)
var glyph_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the glyph texture (single channel alpha)
    let glyph_alpha = textureSample(glyph_texture, glyph_sampler, in.tex_coords).r;
    
    // Blend foreground over background based on glyph alpha
    let final_color = mix(in.bg_color, in.color, glyph_alpha);
    
    return final_color;
}
