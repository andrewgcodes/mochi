//! GPU-accelerated terminal renderer using wgpu
//!
//! Uses instanced rendering with two pipelines:
//! - Rect pipeline: cell backgrounds, tab bar, scrollbar, cursor
//! - Text pipeline: alpha-blended glyph quads from a texture atlas

use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings};
use terminal_core::{Color, Screen, Selection};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::config::ColorScheme;

const ATLAS_SIZE: u32 = 2048;
const INITIAL_RECT_CAPACITY: usize = 8192;
const INITIAL_GLYPH_CAPACITY: usize = 8192;

const RECT_SHADER: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @location(0) rect_pos: vec2<f32>,
    @location(1) rect_size: vec2<f32>,
    @location(2) rect_color: vec4<f32>,
) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );

    let corner = corners[vi];
    let pixel_pos = rect_pos + corner * rect_size;
    let clip_pos = vec2<f32>(
        pixel_pos.x / uniforms.screen_size.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / uniforms.screen_size.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(clip_pos, 0.0, 1.0);
    output.color = rect_color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

const TEXT_SHADER: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_samp: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @location(0) glyph_pos: vec2<f32>,
    @location(1) glyph_size: vec2<f32>,
    @location(2) glyph_uv_pos: vec2<f32>,
    @location(3) glyph_uv_size: vec2<f32>,
    @location(4) glyph_color: vec4<f32>,
) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );

    let corner = corners[vi];
    let pixel_pos = glyph_pos + corner * glyph_size;
    let clip_pos = vec2<f32>(
        pixel_pos.x / uniforms.screen_size.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / uniforms.screen_size.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(clip_pos, 0.0, 1.0);
    output.uv = glyph_uv_pos + corner * glyph_uv_size;
    output.color = glyph_color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_tex, atlas_samp, input.uv).r;
    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}
"#;

pub struct TabInfo<'a> {
    pub title: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct CellSize {
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RectInstance {
    pos: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GlyphInstance {
    pos: [f32; 2],
    size: [f32; 2],
    uv_pos: [f32; 2],
    uv_size: [f32; 2],
    color: [f32; 4],
}

struct GlyphAtlasEntry {
    uv_x: f32,
    uv_y: f32,
    uv_w: f32,
    uv_h: f32,
    width: u32,
    height: u32,
    xmin: i32,
    ymin: i32,
}

struct GlyphAtlas {
    entries: HashMap<(char, bool), GlyphAtlasEntry>,
    width: u32,
    height: u32,
    data: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    dirty: bool,
}

impl GlyphAtlas {
    fn new(width: u32, height: u32) -> Self {
        Self {
            entries: HashMap::with_capacity(256),
            width,
            height,
            data: vec![0u8; (width * height) as usize],
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            dirty: false,
        }
    }

    fn insert(
        &mut self,
        key: (char, bool),
        bitmap: &[u8],
        glyph_w: u32,
        glyph_h: u32,
        xmin: i32,
        ymin: i32,
    ) -> bool {
        if glyph_w == 0 || glyph_h == 0 {
            self.entries.insert(
                key,
                GlyphAtlasEntry {
                    uv_x: 0.0,
                    uv_y: 0.0,
                    uv_w: 0.0,
                    uv_h: 0.0,
                    width: 0,
                    height: 0,
                    xmin,
                    ymin,
                },
            );
            return true;
        }

        let padding = 1u32;
        let padded_w = glyph_w + padding;
        let padded_h = glyph_h + padding;

        if self.cursor_x + padded_w > self.width {
            self.cursor_x = 0;
            self.cursor_y += self.row_height + padding;
            self.row_height = 0;
        }

        if self.cursor_y + padded_h > self.height {
            return false;
        }

        let x = self.cursor_x;
        let y = self.cursor_y;

        for row in 0..glyph_h {
            let dst_start = ((y + row) * self.width + x) as usize;
            let src_start = (row * glyph_w) as usize;
            let len = glyph_w as usize;
            if dst_start + len <= self.data.len() && src_start + len <= bitmap.len() {
                self.data[dst_start..dst_start + len]
                    .copy_from_slice(&bitmap[src_start..src_start + len]);
            }
        }

        let fw = self.width as f32;
        let fh = self.height as f32;

        self.entries.insert(
            key,
            GlyphAtlasEntry {
                uv_x: x as f32 / fw,
                uv_y: y as f32 / fh,
                uv_w: glyph_w as f32 / fw,
                uv_h: glyph_h as f32 / fh,
                width: glyph_w,
                height: glyph_h,
                xmin,
                ymin,
            },
        );

        self.cursor_x += padded_w;
        if padded_h > self.row_height {
            self.row_height = padded_h;
        }
        self.dirty = true;
        true
    }

    fn contains(&self, key: &(char, bool)) -> bool {
        self.entries.contains_key(key)
    }

    fn get(&self, key: &(char, bool)) -> Option<&GlyphAtlasEntry> {
        self.entries.get(key)
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.data.fill(0);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
        self.dirty = true;
    }
}

pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    rect_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,

    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    atlas: GlyphAtlas,
    atlas_texture: wgpu::Texture,
    #[allow(dead_code)]
    atlas_texture_view: wgpu::TextureView,
    atlas_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    atlas_bind_group_layout: wgpu::BindGroupLayout,

    rect_buffer: wgpu::Buffer,
    rect_buffer_capacity: usize,
    text_buffer: wgpu::Buffer,
    text_buffer_capacity: usize,

    font: Font,
    bold_font: Option<Font>,
    bold_font_loaded: bool,
    fallback_fonts: Vec<Font>,
    fallback_fonts_loaded: bool,

    cell_size: CellSize,
    colors: ColorScheme,
    width: u32,
    height: u32,
    font_size: f32,
}

impl GpuRenderer {
    pub fn new(
        window: Arc<Window>,
        font_size: f32,
        colors: ColorScheme,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let scaled_font_size = font_size * scale_factor;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or("Failed to find a suitable GPU adapter")?;

        log::info!(
            "GPU adapter: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend
        );

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Mochi Terminal"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniforms = Uniforms {
            screen_size: [size.width.max(1) as f32, size.height.max(1) as f32],
            _padding: [0.0; 2],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform BG"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let atlas = GlyphAtlas::new(ATLAS_SIZE, ATLAS_SIZE);
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_texture_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Atlas Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let atlas_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Atlas BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atlas BG"),
            layout: &atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
        });

        let rect_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rect Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let rect_instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 8,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 2,
                },
            ],
        };

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[rect_instance_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(TEXT_SHADER.into()),
        });

        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &atlas_bind_group_layout],
            push_constant_ranges: &[],
        });

        let text_instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlyphInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 8,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 16,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 24,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 4,
                },
            ],
        };

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vs_main"),
                buffers: &[text_instance_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let rect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rect Instance Buffer"),
            size: (INITIAL_RECT_CAPACITY * std::mem::size_of::<RectInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Instance Buffer"),
            size: (INITIAL_GLYPH_CAPACITY * std::mem::size_of::<GlyphInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let font_data = include_bytes!("../assets/DejaVuSansMono.ttf");
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())?;

        let metrics = font.metrics('M', scaled_font_size);
        let cell_size = CellSize {
            width: metrics.advance_width.ceil(),
            height: (scaled_font_size * 1.4).ceil(),
            baseline: scaled_font_size,
        };

        let mut renderer = Self {
            device,
            queue,
            surface,
            surface_config,
            rect_pipeline,
            text_pipeline,
            uniform_buffer,
            uniform_bind_group,
            atlas,
            atlas_texture,
            atlas_texture_view,
            atlas_bind_group,
            atlas_bind_group_layout,
            rect_buffer,
            rect_buffer_capacity: INITIAL_RECT_CAPACITY,
            text_buffer,
            text_buffer_capacity: INITIAL_GLYPH_CAPACITY,
            font,
            bold_font: None,
            bold_font_loaded: false,
            fallback_fonts: Vec::new(),
            fallback_fonts_loaded: false,
            cell_size,
            colors,
            width: size.width,
            height: size.height,
            font_size: scaled_font_size,
        };

        renderer.precache_ascii_glyphs();

        Ok(renderer)
    }

    pub fn cell_size(&self) -> CellSize {
        self.cell_size
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        let metrics = self.font.metrics('M', font_size);
        self.cell_size = CellSize {
            width: metrics.advance_width.ceil(),
            height: (font_size * 1.4).ceil(),
            baseline: font_size,
        };
        self.atlas.clear();
        self.precache_ascii_glyphs();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn set_colors(&mut self, colors: ColorScheme) {
        self.colors = colors;
    }

    pub fn render(
        &mut self,
        screen: &Screen,
        selection: &Selection,
        scroll_offset: usize,
        tab_bar_height: u32,
        tabs: &[TabInfo<'_>],
        active_tab: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let width = self.width;
        let height = self.height;
        if width == 0 || height == 0 {
            return Ok(());
        }

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                screen_size: [width as f32, height as f32],
                _padding: [0.0; 2],
            }]),
        );

        let bg_color = self.colors.background_rgb();
        let fg_color = self.colors.foreground_rgb();
        let sel_color = self.colors.selection_rgb();
        let cursor_color = self.colors.cursor_rgb();
        let cell_w = self.cell_size.width;
        let cell_h = self.cell_size.height;
        let baseline = self.cell_size.baseline;

        let cols = screen.cols();
        let rows = screen.rows();
        let scrollback = screen.scrollback();
        let scrollback_len = scrollback.len();

        self.ensure_glyphs_for_tabs(tabs);
        self.ensure_glyphs_for_screen(screen, scroll_offset);

        if self.atlas.dirty {
            self.upload_atlas();
        }

        let mut rects: Vec<RectInstance> = Vec::with_capacity(rows * cols + 64);
        let mut glyphs: Vec<GlyphInstance> = Vec::with_capacity(rows * cols);

        if tab_bar_height > 0 && !tabs.is_empty() {
            self.build_tab_bar_instances(
                &mut rects,
                &mut glyphs,
                tabs,
                active_tab,
                tab_bar_height,
                width,
                bg_color,
                fg_color,
            );
        }

        let cursor = screen.cursor();

        for row in 0..rows {
            let (line, is_from_scrollback, actual_screen_row) = if scroll_offset > 0 {
                let scrollback_row = scrollback_len.saturating_sub(scroll_offset) + row;
                if scrollback_row < scrollback_len {
                    if let Some(sb_line) = scrollback.get(scrollback_row) {
                        (sb_line, true, None)
                    } else {
                        continue;
                    }
                } else {
                    let screen_row = scrollback_row - scrollback_len;
                    if screen_row < rows {
                        (screen.line(screen_row), false, Some(screen_row))
                    } else {
                        continue;
                    }
                }
            } else {
                (screen.line(row), false, Some(row))
            };

            for col in 0..cols.min(line.cols()) {
                let cell = line.cell(col);
                if cell.is_continuation() {
                    continue;
                }

                let x = col as f32 * cell_w;
                let y = row as f32 * cell_h + tab_bar_height as f32;

                let is_selected = !selection.is_empty() && selection.contains(col, row as isize);
                let is_cursor_position = !is_from_scrollback
                    && scroll_offset == 0
                    && actual_screen_row == Some(cursor.row)
                    && cursor.col == col;
                let is_solid_cursor = is_cursor_position && cursor.visible;
                let is_outline_cursor = is_cursor_position && !cursor.visible;

                let (fg, bg) = if is_selected {
                    (fg_color, sel_color)
                } else if is_solid_cursor {
                    (bg_color, cursor_color)
                } else {
                    let fg = Self::resolve_color(
                        &self.colors,
                        &cell.attrs.effective_fg(),
                        true,
                        fg_color,
                        bg_color,
                    );
                    let bg = Self::resolve_color(
                        &self.colors,
                        &cell.attrs.effective_bg(),
                        false,
                        fg_color,
                        bg_color,
                    );
                    (fg, bg)
                };

                let cw = cell.width() as f32 * cell_w;
                let ch = cell_h;

                if bg != bg_color || is_selected || is_solid_cursor {
                    rects.push(RectInstance {
                        pos: [x, y],
                        size: [cw, ch],
                        color: rgb_to_f32(bg),
                    });
                }

                let c = cell.display_char();
                if c != ' ' && !cell.is_empty() {
                    if let Some(entry) = self.atlas.get(&(c, cell.attrs.bold)) {
                        if entry.width > 0 && entry.height > 0 {
                            let gx = x + entry.xmin as f32;
                            let gy = y + baseline - entry.ymin as f32 - entry.height as f32;

                            glyphs.push(GlyphInstance {
                                pos: [gx, gy],
                                size: [entry.width as f32, entry.height as f32],
                                uv_pos: [entry.uv_x, entry.uv_y],
                                uv_size: [entry.uv_w, entry.uv_h],
                                color: rgb_to_f32(fg),
                            });
                        }
                    }
                }

                if is_outline_cursor {
                    let thickness = 2.0f32;
                    let cc = rgb_to_f32(cursor_color);
                    rects.push(RectInstance {
                        pos: [x, y],
                        size: [cw, thickness],
                        color: cc,
                    });
                    rects.push(RectInstance {
                        pos: [x, y + ch - thickness],
                        size: [cw, thickness],
                        color: cc,
                    });
                    rects.push(RectInstance {
                        pos: [x, y + thickness],
                        size: [thickness, ch - 2.0 * thickness],
                        color: cc,
                    });
                    rects.push(RectInstance {
                        pos: [x + cw - thickness, y + thickness],
                        size: [thickness, ch - 2.0 * thickness],
                        color: cc,
                    });
                }
            }
        }

        if scrollback_len > 0 {
            self.build_scrollbar_instances(
                &mut rects,
                scroll_offset,
                scrollback_len,
                rows,
                width,
                height,
                tab_bar_height,
            );
        }

        self.ensure_buffer_capacity(&rects, &glyphs);

        if !rects.is_empty() {
            self.queue
                .write_buffer(&self.rect_buffer, 0, bytemuck::cast_slice(&rects));
        }
        if !glyphs.is_empty() {
            self.queue
                .write_buffer(&self.text_buffer, 0, bytemuck::cast_slice(&glyphs));
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let bg_f = rgb_to_f64(bg_color);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg_f.0,
                            g: bg_f.1,
                            b: bg_f.2,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if !rects.is_empty() {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.rect_buffer.slice(..));
                pass.draw(0..6, 0..rects.len() as u32);
            }

            if !glyphs.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_bind_group(1, &self.atlas_bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_buffer.slice(..));
                pass.draw(0..6, 0..glyphs.len() as u32);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn precache_ascii_glyphs(&mut self) {
        for c in ' '..='~' {
            self.ensure_glyph_cached(c, false);
        }
    }

    fn ensure_glyph_cached(&mut self, c: char, bold: bool) {
        let key = (c, bold);
        if self.atlas.contains(&key) {
            return;
        }

        if bold && !self.bold_font_loaded {
            self.bold_font_loaded = true;
            let bold_font_data = include_bytes!("../assets/DejaVuSansMono-Bold.ttf");
            self.bold_font =
                Font::from_bytes(bold_font_data as &[u8], FontSettings::default()).ok();
        }

        if !self.fallback_fonts_loaded {
            self.fallback_fonts_loaded = true;
            self.load_fallback_fonts();
        }

        let font = if bold {
            self.bold_font.as_ref().unwrap_or(&self.font)
        } else {
            &self.font
        };

        let has_glyph = font.lookup_glyph_index(c) != 0;

        let (metrics, bitmap) = if has_glyph {
            font.rasterize(c, self.font_size)
        } else {
            let mut found = None;
            for fallback in &self.fallback_fonts {
                if fallback.lookup_glyph_index(c) != 0 {
                    found = Some(fallback.rasterize(c, self.font_size));
                    break;
                }
            }
            found.unwrap_or_else(|| font.rasterize(c, self.font_size))
        };

        self.atlas.insert(
            key,
            &bitmap,
            metrics.width as u32,
            metrics.height as u32,
            metrics.xmin,
            metrics.ymin,
        );
    }

    fn load_fallback_fonts(&mut self) {
        let fallback_paths: &[&str] = if cfg!(target_os = "macos") {
            &[
                "/System/Library/Fonts/Apple Color Emoji.ttc",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                "/Library/Fonts/Arial Unicode.ttf",
                "/System/Library/Fonts/Supplemental/Symbola.ttf",
            ]
        } else {
            &[
                "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
                "/usr/share/fonts/noto-emoji/NotoColorEmoji.ttf",
                "/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/unifont/unifont.ttf",
                "/usr/share/fonts/unifont/unifont.ttf",
            ]
        };

        for path in fallback_paths {
            if let Ok(data) = std::fs::read(path) {
                if let Ok(font) = Font::from_bytes(data, FontSettings::default()) {
                    self.fallback_fonts.push(font);
                    log::debug!("Loaded fallback font: {}", path);
                }
            }
        }
    }

    fn ensure_glyphs_for_tabs(&mut self, tabs: &[TabInfo<'_>]) {
        for tab in tabs {
            for c in tab.title.chars() {
                if c != ' ' {
                    self.ensure_glyph_cached(c, false);
                }
            }
        }
        self.ensure_glyph_cached('+', false);
        self.ensure_glyph_cached('x', false);
    }

    fn ensure_glyphs_for_screen(&mut self, screen: &Screen, scroll_offset: usize) {
        let cols = screen.cols();
        let rows = screen.rows();
        let scrollback = screen.scrollback();
        let scrollback_len = scrollback.len();

        for row in 0..rows {
            let line = if scroll_offset > 0 {
                let scrollback_row = scrollback_len.saturating_sub(scroll_offset) + row;
                if scrollback_row < scrollback_len {
                    if let Some(sb_line) = scrollback.get(scrollback_row) {
                        for col in 0..cols.min(sb_line.cols()) {
                            let cell = sb_line.cell(col);
                            if !cell.is_continuation() && !cell.is_empty() {
                                let c = cell.display_char();
                                if c != ' ' {
                                    self.ensure_glyph_cached(c, cell.attrs.bold);
                                }
                            }
                        }
                    }
                    continue;
                } else {
                    let screen_row = scrollback_row - scrollback_len;
                    if screen_row < rows {
                        screen.line(screen_row)
                    } else {
                        continue;
                    }
                }
            } else {
                screen.line(row)
            };

            for col in 0..cols {
                let cell = line.cell(col);
                if !cell.is_continuation() && !cell.is_empty() {
                    let c = cell.display_char();
                    if c != ' ' {
                        self.ensure_glyph_cached(c, cell.attrs.bold);
                    }
                }
            }
        }
    }

    fn upload_atlas(&mut self) {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.atlas.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.atlas.width),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: self.atlas.width,
                height: self.atlas.height,
                depth_or_array_layers: 1,
            },
        );
        self.atlas.dirty = false;
    }

    fn ensure_buffer_capacity(&mut self, rects: &[RectInstance], glyphs: &[GlyphInstance]) {
        if rects.len() > self.rect_buffer_capacity {
            let new_cap = rects.len().next_power_of_two();
            self.rect_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Rect Instance Buffer"),
                size: (new_cap * std::mem::size_of::<RectInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.rect_buffer_capacity = new_cap;
        }

        if glyphs.len() > self.text_buffer_capacity {
            let new_cap = glyphs.len().next_power_of_two();
            self.text_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Text Instance Buffer"),
                size: (new_cap * std::mem::size_of::<GlyphInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.text_buffer_capacity = new_cap;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_tab_bar_instances(
        &self,
        rects: &mut Vec<RectInstance>,
        glyphs: &mut Vec<GlyphInstance>,
        tabs: &[TabInfo<'_>],
        active_tab: usize,
        tab_bar_height: u32,
        buf_width: u32,
        bg_color: (u8, u8, u8),
        fg_color: (u8, u8, u8),
    ) {
        let tab_padding: u32 = 10;
        let close_btn_width: u32 = 20;
        let new_tab_btn_width: u32 = 32;
        let tab_max_width: u32 = 200;

        let tab_bar_bg = blend_color(bg_color, (0, 0, 0), 0.3);
        let active_tab_bg = bg_color;
        let inactive_tab_bg = blend_color(tab_bar_bg, bg_color, 0.3);
        let inactive_fg = blend_color(fg_color, bg_color, 0.4);
        let separator_color = blend_color(bg_color, (128, 128, 128), 0.3);
        let close_color = blend_color(fg_color, (200, 80, 80), 0.5);

        rects.push(RectInstance {
            pos: [0.0, 0.0],
            size: [buf_width as f32, tab_bar_height as f32],
            color: rgb_to_f32(tab_bar_bg),
        });

        let num_tabs = tabs.len() as u32;
        let available_width = buf_width.saturating_sub(new_tab_btn_width);
        let tab_width = if num_tabs > 0 {
            (available_width / num_tabs).min(tab_max_width)
        } else {
            tab_max_width
        };

        for (i, tab) in tabs.iter().enumerate() {
            let is_active = i == active_tab;
            let tab_x = (i as u32 * tab_width) as f32;
            let tab_bg = if is_active {
                active_tab_bg
            } else {
                inactive_tab_bg
            };
            let text_color = if is_active { fg_color } else { inactive_fg };

            rects.push(RectInstance {
                pos: [tab_x, 0.0],
                size: [tab_width as f32, tab_bar_height as f32],
                color: rgb_to_f32(tab_bg),
            });

            if is_active {
                let accent = blend_color(fg_color, (100, 149, 237), 0.5);
                rects.push(RectInstance {
                    pos: [tab_x, (tab_bar_height - 2) as f32],
                    size: [tab_width as f32, 2.0],
                    color: rgb_to_f32(accent),
                });
            }

            if i < tabs.len() - 1 {
                rects.push(RectInstance {
                    pos: [tab_x + tab_width as f32 - 1.0, 4.0],
                    size: [1.0, (tab_bar_height - 8) as f32],
                    color: rgb_to_f32(separator_color),
                });
            }

            let text_x = tab_x + tab_padding as f32;
            let text_y = ((tab_bar_height as f32 - self.cell_size.height) / 2.0).max(0.0);
            let max_text_width = tab_width.saturating_sub(tab_padding * 2 + close_btn_width) as f32;

            self.build_text_glyphs(
                glyphs,
                tab.title,
                text_x,
                text_y,
                text_color,
                max_text_width,
            );

            if tabs.len() > 1 {
                let close_x = tab_x + tab_width as f32 - close_btn_width as f32;
                let close_y = text_y;
                if let Some(entry) = self.atlas.get(&('x', false)) {
                    if entry.width > 0 && entry.height > 0 {
                        let gx = close_x + entry.xmin as f32;
                        let gy = close_y + self.cell_size.baseline
                            - entry.ymin as f32
                            - entry.height as f32;
                        glyphs.push(GlyphInstance {
                            pos: [gx, gy],
                            size: [entry.width as f32, entry.height as f32],
                            uv_pos: [entry.uv_x, entry.uv_y],
                            uv_size: [entry.uv_w, entry.uv_h],
                            color: rgb_to_f32(close_color),
                        });
                    }
                }
            }
        }

        let plus_btn_x = (num_tabs * tab_width) as f32;
        let plus_text_x = plus_btn_x + ((new_tab_btn_width as f32 - self.cell_size.width) / 2.0);
        let plus_text_y = ((tab_bar_height as f32 - self.cell_size.height) / 2.0).max(0.0);
        let plus_bg = blend_color(tab_bar_bg, bg_color, 0.15);

        rects.push(RectInstance {
            pos: [plus_btn_x, 0.0],
            size: [new_tab_btn_width as f32, tab_bar_height as f32],
            color: rgb_to_f32(plus_bg),
        });

        if let Some(entry) = self.atlas.get(&('+', false)) {
            if entry.width > 0 && entry.height > 0 {
                let gx = plus_text_x + entry.xmin as f32;
                let gy =
                    plus_text_y + self.cell_size.baseline - entry.ymin as f32 - entry.height as f32;
                glyphs.push(GlyphInstance {
                    pos: [gx, gy],
                    size: [entry.width as f32, entry.height as f32],
                    uv_pos: [entry.uv_x, entry.uv_y],
                    uv_size: [entry.uv_w, entry.uv_h],
                    color: rgb_to_f32(fg_color),
                });
            }
        }

        rects.push(RectInstance {
            pos: [0.0, (tab_bar_height - 1) as f32],
            size: [buf_width as f32, 1.0],
            color: rgb_to_f32(separator_color),
        });
    }

    fn build_text_glyphs(
        &self,
        glyphs: &mut Vec<GlyphInstance>,
        text: &str,
        x: f32,
        y: f32,
        color: (u8, u8, u8),
        max_width: f32,
    ) {
        let mut cx = x;
        for ch in text.chars() {
            if cx - x >= max_width {
                break;
            }
            if ch != ' ' {
                if let Some(entry) = self.atlas.get(&(ch, false)) {
                    if entry.width > 0 && entry.height > 0 {
                        let gx = cx + entry.xmin as f32;
                        let gy =
                            y + self.cell_size.baseline - entry.ymin as f32 - entry.height as f32;
                        glyphs.push(GlyphInstance {
                            pos: [gx, gy],
                            size: [entry.width as f32, entry.height as f32],
                            uv_pos: [entry.uv_x, entry.uv_y],
                            uv_size: [entry.uv_w, entry.uv_h],
                            color: rgb_to_f32(color),
                        });
                    }
                }
            }
            cx += self.cell_size.width;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_scrollbar_instances(
        &self,
        rects: &mut Vec<RectInstance>,
        scroll_offset: usize,
        scrollback_len: usize,
        visible_rows: usize,
        buf_width: u32,
        buf_height: u32,
        tab_bar_height: u32,
    ) {
        let scrollbar_width = 12.0f32;
        let scrollbar_x = buf_width as f32 - scrollbar_width;
        let scrollbar_height = buf_height.saturating_sub(tab_bar_height) as f32;
        let y_off = tab_bar_height as f32;

        let total_lines = scrollback_len + visible_rows;

        let thumb_height =
            ((visible_rows as f32 / total_lines as f32) * scrollbar_height).max(20.0);

        let scroll_range = scrollbar_height - thumb_height;
        let thumb_y = if scrollback_len > 0 {
            (scrollback_len - scroll_offset) as f32 / scrollback_len as f32 * scroll_range
        } else {
            scroll_range
        };

        let track_color = (40, 40, 40);
        rects.push(RectInstance {
            pos: [scrollbar_x, y_off],
            size: [scrollbar_width, scrollbar_height],
            color: rgb_to_f32(track_color),
        });

        let thumb_color = if scroll_offset > 0 {
            (120, 120, 120)
        } else {
            (80, 80, 80)
        };
        rects.push(RectInstance {
            pos: [scrollbar_x + 1.0, y_off + thumb_y],
            size: [scrollbar_width - 2.0, thumb_height],
            color: rgb_to_f32(thumb_color),
        });
    }

    fn resolve_color(
        colors: &ColorScheme,
        color: &Color,
        is_fg: bool,
        fg_default: (u8, u8, u8),
        bg_default: (u8, u8, u8),
    ) -> (u8, u8, u8) {
        match color {
            Color::Default => {
                if is_fg {
                    fg_default
                } else {
                    bg_default
                }
            }
            Color::Indexed(idx) => {
                if *idx < 16 {
                    colors.ansi_rgb(*idx as usize)
                } else {
                    color.to_rgb()
                }
            }
            Color::Rgb { r, g, b } => (*r, *g, *b),
        }
    }
}

fn rgb_to_f32(c: (u8, u8, u8)) -> [f32; 4] {
    [
        c.0 as f32 / 255.0,
        c.1 as f32 / 255.0,
        c.2 as f32 / 255.0,
        1.0,
    ]
}

fn rgb_to_f64(c: (u8, u8, u8)) -> (f64, f64, f64) {
    (c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0)
}

fn blend_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    (
        (a.0 as f32 * (1.0 - t) + b.0 as f32 * t) as u8,
        (a.1 as f32 * (1.0 - t) + b.1 as f32 * t) as u8,
        (a.2 as f32 * (1.0 - t) + b.2 as f32 * t) as u8,
    )
}
