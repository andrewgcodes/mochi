//! GPU-accelerated terminal renderer using wgpu
//!
//! This module provides the graphical rendering for the terminal emulator.
//! It uses wgpu for GPU-accelerated rendering and fontdue for font rasterization.

use std::collections::HashMap;
use std::sync::Arc;

use crate::core::{Color, Screen};

/// Configuration for the renderer
#[derive(Debug, Clone)]
pub struct RendererConfig {
    /// Font size in pixels
    pub font_size: f32,
    /// Line height multiplier (1.0 = normal)
    pub line_height: f32,
    /// Default foreground color
    pub default_fg: [u8; 3],
    /// Default background color
    pub default_bg: [u8; 3],
    /// Cursor color
    pub cursor_color: [u8; 3],
    /// Selection color (with alpha)
    pub selection_color: [u8; 4],
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            line_height: 1.2,
            default_fg: [204, 204, 204],           // Light gray
            default_bg: [30, 30, 30],              // Dark gray
            cursor_color: [255, 255, 255],         // White
            selection_color: [100, 100, 200, 128], // Blue with alpha
        }
    }
}

/// Cached glyph information
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// Texture coordinates (u, v, width, height) in atlas
    pub tex_coords: [f32; 4],
    /// Glyph metrics
    pub metrics: fontdue::Metrics,
}

/// Glyph cache using a texture atlas
pub struct GlyphCache {
    /// Font for rasterization
    font: fontdue::Font,
    /// Bold font variant
    bold_font: Option<fontdue::Font>,
    /// Italic font variant
    italic_font: Option<fontdue::Font>,
    /// Cached glyphs: (char, bold, italic) -> GlyphInfo
    glyphs: HashMap<(char, bool, bool), GlyphInfo>,
    /// Atlas texture data (RGBA)
    atlas_data: Vec<u8>,
    /// Atlas dimensions
    atlas_width: u32,
    atlas_height: u32,
    /// Current position in atlas
    atlas_x: u32,
    atlas_y: u32,
    /// Current row height in atlas
    row_height: u32,
    /// Font size
    font_size: f32,
    /// Cell dimensions
    cell_width: f32,
    cell_height: f32,
}

impl GlyphCache {
    /// Create a new glyph cache with the given font
    pub fn new(font_data: &[u8], font_size: f32, line_height: f32) -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
            .map_err(|e| format!("Failed to load font: {}", e))?;

        // Calculate cell dimensions from font metrics
        let metrics = font.metrics('M', font_size);
        let cell_width = metrics.advance_width.ceil();
        let cell_height = (font_size * line_height).ceil();

        // Initial atlas size (will grow as needed)
        let atlas_width = 512;
        let atlas_height = 512;
        let atlas_data = vec![0u8; (atlas_width * atlas_height * 4) as usize];

        Ok(Self {
            font,
            bold_font: None,
            italic_font: None,
            glyphs: HashMap::new(),
            atlas_data,
            atlas_width,
            atlas_height,
            atlas_x: 0,
            atlas_y: 0,
            row_height: 0,
            font_size,
            cell_width,
            cell_height,
        })
    }

    /// Get cell dimensions
    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }

    /// Get or create a glyph
    pub fn get_glyph(&mut self, c: char, bold: bool, italic: bool) -> &GlyphInfo {
        let key = (c, bold, italic);
        if !self.glyphs.contains_key(&key) {
            self.rasterize_glyph(c, bold, italic);
        }
        self.glyphs.get(&key).unwrap()
    }

    /// Rasterize a glyph and add it to the atlas
    fn rasterize_glyph(&mut self, c: char, bold: bool, italic: bool) {
        // Select font variant
        let font = if bold && self.bold_font.is_some() {
            self.bold_font.as_ref().unwrap()
        } else if italic && self.italic_font.is_some() {
            self.italic_font.as_ref().unwrap()
        } else {
            &self.font
        };

        let (metrics, bitmap) = font.rasterize(c, self.font_size);

        // Check if we need to move to next row
        if self.atlas_x + metrics.width as u32 > self.atlas_width {
            self.atlas_x = 0;
            self.atlas_y += self.row_height;
            self.row_height = 0;
        }

        // Check if we need to grow the atlas
        if self.atlas_y + metrics.height as u32 > self.atlas_height {
            self.grow_atlas();
        }

        // Copy bitmap to atlas
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let src_idx = y * metrics.width + x;
                let dst_x = self.atlas_x + x as u32;
                let dst_y = self.atlas_y + y as u32;
                let dst_idx = ((dst_y * self.atlas_width + dst_x) * 4) as usize;

                let alpha = bitmap[src_idx];
                self.atlas_data[dst_idx] = 255; // R
                self.atlas_data[dst_idx + 1] = 255; // G
                self.atlas_data[dst_idx + 2] = 255; // B
                self.atlas_data[dst_idx + 3] = alpha; // A
            }
        }

        // Calculate texture coordinates
        let tex_coords = [
            self.atlas_x as f32 / self.atlas_width as f32,
            self.atlas_y as f32 / self.atlas_height as f32,
            metrics.width as f32 / self.atlas_width as f32,
            metrics.height as f32 / self.atlas_height as f32,
        ];

        // Store glyph info
        let glyph_info = GlyphInfo {
            tex_coords,
            metrics,
        };
        self.glyphs.insert((c, bold, italic), glyph_info);

        // Update atlas position
        self.atlas_x += metrics.width as u32 + 1;
        self.row_height = self.row_height.max(metrics.height as u32 + 1);
    }

    /// Grow the atlas texture
    fn grow_atlas(&mut self) {
        let new_height = self.atlas_height * 2;
        let mut new_data = vec![0u8; (self.atlas_width * new_height * 4) as usize];

        // Copy old data
        for y in 0..self.atlas_height {
            let src_start = (y * self.atlas_width * 4) as usize;
            let src_end = src_start + (self.atlas_width * 4) as usize;
            let dst_start = (y * self.atlas_width * 4) as usize;
            new_data[dst_start..dst_start + (self.atlas_width * 4) as usize]
                .copy_from_slice(&self.atlas_data[src_start..src_end]);
        }

        self.atlas_data = new_data;
        self.atlas_height = new_height;

        // Invalidate texture coordinates (would need to update GPU texture)
        // For simplicity, we recalculate on next use
    }

    /// Get the atlas texture data
    pub fn atlas_data(&self) -> &[u8] {
        &self.atlas_data
    }

    /// Get atlas dimensions
    pub fn atlas_size(&self) -> (u32, u32) {
        (self.atlas_width, self.atlas_height)
    }
}

/// Vertex for rendering
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

// Safety: Vertex is a plain-old-data struct with no padding
unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// The terminal renderer
pub struct Renderer {
    /// wgpu surface
    surface: wgpu::Surface<'static>,
    /// wgpu device
    device: wgpu::Device,
    /// wgpu queue
    queue: wgpu::Queue,
    /// Surface configuration
    config: wgpu::SurfaceConfiguration,
    /// Render pipeline
    pipeline: wgpu::RenderPipeline,
    /// Vertex buffer
    vertex_buffer: wgpu::Buffer,
    /// Index buffer
    index_buffer: wgpu::Buffer,
    /// Glyph atlas texture
    atlas_texture: wgpu::Texture,
    /// Atlas texture bind group
    atlas_bind_group: wgpu::BindGroup,
    /// Glyph cache
    glyph_cache: GlyphCache,
    /// Renderer configuration
    renderer_config: RendererConfig,
    /// Current window size
    window_size: (u32, u32),
    /// Vertex data buffer
    vertices: Vec<Vertex>,
    /// Index data buffer
    indices: Vec<u32>,
}

impl Renderer {
    /// Create a new renderer
    pub async fn new(
        window: Arc<winit::window::Window>,
        config: RendererConfig,
    ) -> Result<Self, String> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance
            .create_surface(window)
            .map_err(|e| format!("Failed to create surface: {}", e))?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable GPU adapter")?;

        // Request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Load default font (embedded monospace font)
        let font_data = include_bytes!("../../assets/fonts/DejaVuSansMono.ttf");
        let glyph_cache = GlyphCache::new(font_data, config.font_size, config.line_height)?;

        // Create glyph atlas texture
        let (atlas_width, atlas_height) = glyph_cache.atlas_size();
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Create texture sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Atlas Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atlas Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terminal Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terminal Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terminal Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create vertex and index buffers
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1024 * 1024, // 1MB initial size
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 512 * 1024, // 512KB initial size
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config: surface_config,
            pipeline,
            vertex_buffer,
            index_buffer,
            atlas_texture,
            atlas_bind_group,
            glyph_cache,
            renderer_config: config,
            window_size: (size.width, size.height),
            vertices: Vec::new(),
            indices: Vec::new(),
        })
    }

    /// Resize the renderer
    pub fn resize(&mut self, new_size: (u32, u32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.window_size = new_size;
            self.config.width = new_size.0;
            self.config.height = new_size.1;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Get the terminal dimensions in cells
    pub fn terminal_size(&self) -> (u16, u16) {
        let (cell_width, cell_height) = self.glyph_cache.cell_size();
        let cols = (self.window_size.0 as f32 / cell_width).floor() as u16;
        let rows = (self.window_size.1 as f32 / cell_height).floor() as u16;
        (cols.max(1), rows.max(1))
    }

    /// Get cell size in pixels
    pub fn cell_size(&self) -> (f32, f32) {
        self.glyph_cache.cell_size()
    }

    /// Render the terminal screen
    pub fn render(&mut self, screen: &Screen) -> Result<(), String> {
        // Get output texture
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get surface texture: {}", e))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build vertex data
        self.build_vertices(screen);

        // Update vertex buffer
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));

        // Update index buffer
        self.queue
            .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        // Update atlas texture
        let (atlas_width, atlas_height) = self.glyph_cache.atlas_size();
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.glyph_cache.atlas_data(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width * 4),
                rows_per_image: Some(atlas_height),
            },
            wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
        );

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render pass
        {
            let bg = &self.renderer_config.default_bg;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0] as f64 / 255.0,
                            g: bg[1] as f64 / 255.0,
                            b: bg[2] as f64 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.atlas_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Build vertex data from screen
    fn build_vertices(&mut self, screen: &Screen) {
        self.vertices.clear();
        self.indices.clear();

        let (cell_width, cell_height) = self.glyph_cache.cell_size();
        let (width, height) = self.window_size;

        // Convert pixel coordinates to normalized device coordinates
        let to_ndc_x = |x: f32| -> f32 { (x / width as f32) * 2.0 - 1.0 };
        let to_ndc_y = |y: f32| -> f32 { 1.0 - (y / height as f32) * 2.0 };

        let grid = screen.grid();
        let cursor = screen.cursor();

        // Render each cell
        for row in 0..grid.rows() {
            if let Some(line) = grid.line(row) {
                for col in 0..grid.cols() {
                    let Some(cell) = line.cell(col) else {
                        continue;
                    };

                    let x = col as f32 * cell_width;
                    let y = row as f32 * cell_height;

                    // Get cell colors
                    let (fg, bg) = self.get_cell_colors(cell.fg(), cell.bg(), cell.attrs());

                    // Draw background if not default
                    if bg != self.renderer_config.default_bg {
                        self.add_rect(
                            to_ndc_x(x),
                            to_ndc_y(y),
                            to_ndc_x(x + cell_width),
                            to_ndc_y(y + cell_height),
                            [
                                bg[0] as f32 / 255.0,
                                bg[1] as f32 / 255.0,
                                bg[2] as f32 / 255.0,
                                1.0,
                            ],
                            [0.0, 0.0, 0.0, 0.0], // No texture
                        );
                    }

                    // Draw cursor
                    if row == cursor.row() && col == cursor.col() && cursor.is_visible() {
                        let cursor_color = &self.renderer_config.cursor_color;
                        self.add_rect(
                            to_ndc_x(x),
                            to_ndc_y(y),
                            to_ndc_x(x + cell_width),
                            to_ndc_y(y + cell_height),
                            [
                                cursor_color[0] as f32 / 255.0,
                                cursor_color[1] as f32 / 255.0,
                                cursor_color[2] as f32 / 255.0,
                                0.5,
                            ],
                            [0.0, 0.0, 0.0, 0.0],
                        );
                    }

                    // Draw character
                    let content = cell.content();
                    if !content.is_empty() {
                        let c = content.chars().next().unwrap();
                        if c != ' ' {
                            let attrs = cell.attrs();
                            let glyph = self.glyph_cache.get_glyph(c, attrs.bold, attrs.italic);

                            // Clone glyph data to avoid borrow issues
                            let glyph_metrics = glyph.metrics;
                            let glyph_tex_coords = glyph.tex_coords;

                            // Calculate glyph position
                            let glyph_x = x + glyph_metrics.xmin as f32;
                            let glyph_y = y + cell_height
                                - glyph_metrics.height as f32
                                - glyph_metrics.ymin as f32;

                            self.add_rect(
                                to_ndc_x(glyph_x),
                                to_ndc_y(glyph_y),
                                to_ndc_x(glyph_x + glyph_metrics.width as f32),
                                to_ndc_y(glyph_y + glyph_metrics.height as f32),
                                [
                                    fg[0] as f32 / 255.0,
                                    fg[1] as f32 / 255.0,
                                    fg[2] as f32 / 255.0,
                                    1.0,
                                ],
                                glyph_tex_coords,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Get foreground and background colors for a cell
    fn get_cell_colors(
        &self,
        fg_color: Color,
        bg_color: Color,
        attrs: &crate::core::CellAttributes,
    ) -> ([u8; 3], [u8; 3]) {
        // Get base colors
        let mut fg = match fg_color {
            Color::Default => self.renderer_config.default_fg,
            Color::Indexed(idx) => self.indexed_color(idx),
            Color::Rgb(r, g, b) => [r, g, b],
        };

        let mut bg = match bg_color {
            Color::Default => self.renderer_config.default_bg,
            Color::Indexed(idx) => self.indexed_color(idx),
            Color::Rgb(r, g, b) => [r, g, b],
        };

        // Handle inverse
        if attrs.inverse {
            std::mem::swap(&mut fg, &mut bg);
        }

        // Handle faint (dim)
        if attrs.faint {
            fg = [fg[0] / 2, fg[1] / 2, fg[2] / 2];
        }

        // Handle hidden
        if attrs.hidden {
            fg = bg;
        }

        (fg, bg)
    }

    /// Convert indexed color to RGB
    fn indexed_color(&self, idx: u8) -> [u8; 3] {
        match idx {
            // Standard colors (0-7)
            0 => [0, 0, 0],       // Black
            1 => [205, 49, 49],   // Red
            2 => [13, 188, 121],  // Green
            3 => [229, 229, 16],  // Yellow
            4 => [36, 114, 200],  // Blue
            5 => [188, 63, 188],  // Magenta
            6 => [17, 168, 205],  // Cyan
            7 => [229, 229, 229], // White

            // Bright colors (8-15)
            8 => [102, 102, 102],  // Bright Black
            9 => [241, 76, 76],    // Bright Red
            10 => [35, 209, 139],  // Bright Green
            11 => [245, 245, 67],  // Bright Yellow
            12 => [59, 142, 234],  // Bright Blue
            13 => [214, 112, 214], // Bright Magenta
            14 => [41, 184, 219],  // Bright Cyan
            15 => [255, 255, 255], // Bright White

            // 216 color cube (16-231)
            16..=231 => {
                let idx = idx - 16;
                let r = (idx / 36) % 6;
                let g = (idx / 6) % 6;
                let b = idx % 6;
                let to_val = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
                [to_val(r), to_val(g), to_val(b)]
            },

            // Grayscale (232-255)
            232..=255 => {
                let gray = 8 + (idx - 232) * 10;
                [gray, gray, gray]
            },
        }
    }

    /// Add a rectangle to the vertex buffer
    fn add_rect(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 4],
        tex_coords: [f32; 4],
    ) {
        let base_idx = self.vertices.len() as u32;

        // Add vertices (top-left, top-right, bottom-right, bottom-left)
        self.vertices.push(Vertex {
            position: [x1, y1],
            tex_coords: [tex_coords[0], tex_coords[1]],
            color,
        });
        self.vertices.push(Vertex {
            position: [x2, y1],
            tex_coords: [tex_coords[0] + tex_coords[2], tex_coords[1]],
            color,
        });
        self.vertices.push(Vertex {
            position: [x2, y2],
            tex_coords: [tex_coords[0] + tex_coords[2], tex_coords[1] + tex_coords[3]],
            color,
        });
        self.vertices.push(Vertex {
            position: [x1, y2],
            tex_coords: [tex_coords[0], tex_coords[1] + tex_coords[3]],
            color,
        });

        // Add indices for two triangles
        self.indices.push(base_idx);
        self.indices.push(base_idx + 1);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx + 3);
    }
}
