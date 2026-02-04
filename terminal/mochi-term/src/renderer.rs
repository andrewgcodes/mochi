//! GPU-accelerated terminal renderer using wgpu.
//!
//! This module handles:
//! - Window creation and management via winit
//! - GPU rendering pipeline setup via wgpu
//! - Font rasterization and glyph caching via fontdue
//! - Terminal screen rendering with colors and attributes

use fontdue::{Font, FontSettings};
use log::{info, warn};
use mochi_core::{Color, Screen};
use std::collections::HashMap;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Default font size in pixels
const DEFAULT_FONT_SIZE: f32 = 16.0;

/// Default cell padding
const CELL_PADDING: f32 = 2.0;

/// Vertex for rendering textured quads
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 4],
    bg_color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Float32x4,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Cached glyph information
struct GlyphInfo {
    /// Position in texture atlas (x, y, width, height)
    atlas_rect: (u32, u32, u32, u32),
}

/// Glyph cache with texture atlas
struct GlyphCache {
    font: Font,
    font_size: f32,
    glyphs: HashMap<char, GlyphInfo>,
    atlas_data: Vec<u8>,
    atlas_width: u32,
    atlas_height: u32,
    next_x: u32,
    next_y: u32,
    row_height: u32,
    cell_width: f32,
    cell_height: f32,
}

impl GlyphCache {
    fn new(font_data: &[u8], font_size: f32) -> Self {
        let font =
            Font::from_bytes(font_data, FontSettings::default()).expect("Failed to load font");

        let metrics = font.metrics('M', font_size);
        let cell_width = metrics.advance_width.ceil();
        let cell_height = font_size + CELL_PADDING * 2.0;

        let atlas_width = 512;
        let atlas_height = 512;

        GlyphCache {
            font,
            font_size,
            glyphs: HashMap::new(),
            atlas_data: vec![0; (atlas_width * atlas_height) as usize],
            atlas_width,
            atlas_height,
            next_x: 0,
            next_y: 0,
            row_height: 0,
            cell_width,
            cell_height,
        }
    }

    fn get_or_rasterize(&mut self, c: char) -> &GlyphInfo {
        if !self.glyphs.contains_key(&c) {
            self.rasterize(c);
        }
        self.glyphs.get(&c).unwrap()
    }

    fn rasterize(&mut self, c: char) {
        let (metrics, bitmap) = self.font.rasterize(c, self.font_size);

        let glyph_width = metrics.width as u32;
        let glyph_height = metrics.height as u32;

        if self.next_x + glyph_width > self.atlas_width {
            self.next_x = 0;
            self.next_y += self.row_height + 1;
            self.row_height = 0;
        }

        if self.next_y + glyph_height > self.atlas_height {
            warn!("Glyph atlas full, some characters may not render");
            return;
        }

        for y in 0..glyph_height {
            for x in 0..glyph_width {
                let src_idx = (y * glyph_width + x) as usize;
                let dst_idx = ((self.next_y + y) * self.atlas_width + self.next_x + x) as usize;
                if src_idx < bitmap.len() && dst_idx < self.atlas_data.len() {
                    self.atlas_data[dst_idx] = bitmap[src_idx];
                }
            }
        }

        let glyph_info = GlyphInfo {
            atlas_rect: (self.next_x, self.next_y, glyph_width, glyph_height),
        };

        self.glyphs.insert(c, glyph_info);

        self.next_x += glyph_width + 1;
        self.row_height = self.row_height.max(glyph_height);
    }
}

/// Terminal renderer state
pub struct Renderer {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    pub config: Option<wgpu::SurfaceConfiguration>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    glyph_texture: Option<wgpu::Texture>,
    glyph_bind_group: Option<wgpu::BindGroup>,
    glyph_cache: Option<GlyphCache>,

    pub cols: usize,
    pub rows: usize,

    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Renderer {
    pub fn new(cols: usize, rows: usize) -> Self {
        Renderer {
            window: None,
            surface: None,
            device: None,
            queue: None,
            config: None,
            render_pipeline: None,
            vertex_buffer: None,
            index_buffer: None,
            glyph_texture: None,
            glyph_bind_group: None,
            glyph_cache: None,
            cols,
            rows,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn initialize_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Terminal Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
        let glyph_cache = GlyphCache::new(font_data, DEFAULT_FONT_SIZE);

        let glyph_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: glyph_cache.atlas_width,
                height: glyph_cache.atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let glyph_texture_view = glyph_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let glyph_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Glyph Bind Group Layout"),
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

        let glyph_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Glyph Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&glyph_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&glyph_sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terminal Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terminal Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terminal Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1024 * 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: 512 * 1024,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.render_pipeline = Some(render_pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.glyph_texture = Some(glyph_texture);
        self.glyph_bind_group = Some(glyph_bind_group);
        self.glyph_cache = Some(glyph_cache);

        info!("GPU renderer initialized");
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            if let (Some(config), Some(surface), Some(device)) =
                (&mut self.config, &self.surface, &self.device)
            {
                config.width = new_size.width;
                config.height = new_size.height;
                surface.configure(device, config);
            }
        }
    }

    pub fn update_screen(&mut self, screen: &Screen) {
        self.vertices.clear();
        self.indices.clear();

        let config = match &self.config {
            Some(c) => c,
            None => return,
        };

        let screen_width = config.width as f32;
        let screen_height = config.height as f32;

        let glyph_cache = match &mut self.glyph_cache {
            Some(cache) => cache,
            None => return,
        };

        let cell_width = glyph_cache.cell_width;
        let cell_height = glyph_cache.cell_height;

        let atlas_width = glyph_cache.atlas_width as f32;
        let atlas_height = glyph_cache.atlas_height as f32;

        for row in 0..screen.rows().min(self.rows) {
            if let Some(line) = screen.get_line(row) {
                for col in 0..screen.cols().min(self.cols) {
                    let cell = match line.get(col) {
                        Some(c) => c,
                        None => continue,
                    };
                    let c = cell.character;

                    let x = (col as f32 * cell_width / screen_width) * 2.0 - 1.0;
                    let y = 1.0 - ((row as f32 + 1.0) * cell_height / screen_height) * 2.0;
                    let w = (cell_width / screen_width) * 2.0;
                    let h = (cell_height / screen_height) * 2.0;

                    let (fg_color, bg_color) = get_cell_colors(cell);

                    let base_idx = self.vertices.len() as u16;

                    let glyph = glyph_cache.get_or_rasterize(c);
                    let (tx, ty, tw, th) = glyph.atlas_rect;

                    let u0 = tx as f32 / atlas_width;
                    let v0 = ty as f32 / atlas_height;
                    let u1 = (tx + tw) as f32 / atlas_width;
                    let v1 = (ty + th) as f32 / atlas_height;

                    self.vertices.push(Vertex {
                        position: [x, y + h],
                        tex_coords: [u0, v0],
                        color: fg_color,
                        bg_color,
                    });
                    self.vertices.push(Vertex {
                        position: [x + w, y + h],
                        tex_coords: [u1, v0],
                        color: fg_color,
                        bg_color,
                    });
                    self.vertices.push(Vertex {
                        position: [x + w, y],
                        tex_coords: [u1, v1],
                        color: fg_color,
                        bg_color,
                    });
                    self.vertices.push(Vertex {
                        position: [x, y],
                        tex_coords: [u0, v1],
                        color: fg_color,
                        bg_color,
                    });

                    self.indices.push(base_idx);
                    self.indices.push(base_idx + 1);
                    self.indices.push(base_idx + 2);
                    self.indices.push(base_idx);
                    self.indices.push(base_idx + 2);
                    self.indices.push(base_idx + 3);
                }
            }
        }

        let cursor = screen.cursor();
        if cursor.visible {
            let x = (cursor.col as f32 * cell_width / screen_width) * 2.0 - 1.0;
            let y = 1.0 - ((cursor.row as f32 + 1.0) * cell_height / screen_height) * 2.0;
            let w = (cell_width / screen_width) * 2.0;
            let h = (cell_height / screen_height) * 2.0;

            let base_idx = self.vertices.len() as u16;
            let cursor_color = [1.0, 1.0, 1.0, 0.7];

            self.vertices.push(Vertex {
                position: [x, y + h],
                tex_coords: [0.0, 0.0],
                color: cursor_color,
                bg_color: cursor_color,
            });
            self.vertices.push(Vertex {
                position: [x + w, y + h],
                tex_coords: [0.0, 0.0],
                color: cursor_color,
                bg_color: cursor_color,
            });
            self.vertices.push(Vertex {
                position: [x + w, y],
                tex_coords: [0.0, 0.0],
                color: cursor_color,
                bg_color: cursor_color,
            });
            self.vertices.push(Vertex {
                position: [x, y],
                tex_coords: [0.0, 0.0],
                color: cursor_color,
                bg_color: cursor_color,
            });

            self.indices.push(base_idx);
            self.indices.push(base_idx + 1);
            self.indices.push(base_idx + 2);
            self.indices.push(base_idx);
            self.indices.push(base_idx + 2);
            self.indices.push(base_idx + 3);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let surface = self.surface.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        let render_pipeline = self.render_pipeline.as_ref().unwrap();
        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let index_buffer = self.index_buffer.as_ref().unwrap();
        let glyph_bind_group = self.glyph_bind_group.as_ref().unwrap();
        let glyph_cache = self.glyph_cache.as_ref().unwrap();
        let glyph_texture = self.glyph_texture.as_ref().unwrap();
        let device = self.device.as_ref().unwrap();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: glyph_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &glyph_cache.atlas_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(glyph_cache.atlas_width),
                rows_per_image: Some(glyph_cache.atlas_height),
            },
            wgpu::Extent3d {
                width: glyph_cache.atlas_width,
                height: glyph_cache.atlas_height,
                depth_or_array_layers: 1,
            },
        );

        if !self.vertices.is_empty() {
            queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }

        if !self.indices.is_empty() {
            queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(&self.indices));
        }

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, glyph_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub fn cell_dimensions(&self) -> Option<(f32, f32)> {
        self.glyph_cache
            .as_ref()
            .map(|cache| (cache.cell_width, cache.cell_height))
    }

    pub fn calculate_terminal_size(&self) -> Option<(usize, usize)> {
        let config = self.config.as_ref()?;
        let (cell_width, cell_height) = self.cell_dimensions()?;

        let cols = (config.width as f32 / cell_width).floor() as usize;
        let rows = (config.height as f32 / cell_height).floor() as usize;

        Some((cols.max(1), rows.max(1)))
    }
}

/// Application state for winit event loop
pub struct TerminalApp {
    pub renderer: Renderer,
    pub screen: Screen,
    pub parser: mochi_parser::Parser,
    pub performer: crate::performer::Performer,
    pub pty: Option<mochi_pty::Pty>,
    pub input_encoder: crate::input::InputEncoder,
    pub bracketed_paste_mode: bool,
    pub modifiers: winit::keyboard::ModifiersState,
    pub clipboard: Option<arboard::Clipboard>,
    pub selection: mochi_core::Selection,
    pub mouse_pressed: bool,
    pub scroll_offset: i64,
    pub mouse_col: u16,
    pub mouse_row: u16,
}

impl ApplicationHandler for TerminalApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.window.is_none() {
            let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
            let glyph_cache = GlyphCache::new(font_data, DEFAULT_FONT_SIZE);

            let window_width = (self.renderer.cols as f32 * glyph_cache.cell_width) as u32;
            let window_height = (self.renderer.rows as f32 * glyph_cache.cell_height) as u32;

            let window_attributes = Window::default_attributes()
                .with_title("Mochi Terminal")
                .with_inner_size(LogicalSize::new(window_width, window_height));

            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create window"),
            );

            self.renderer.initialize_gpu(window);
            self.renderer.update_screen(&self.screen);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Window close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.renderer.resize(physical_size);

                if let Some((cols, rows)) = self.renderer.calculate_terminal_size() {
                    if cols != self.renderer.cols || rows != self.renderer.rows {
                        self.renderer.cols = cols;
                        self.renderer.rows = rows;
                        self.screen = Screen::new(cols, rows);

                        if let Some(pty) = &self.pty {
                            let _ = pty.set_size(mochi_pty::PtySize::new(cols as u16, rows as u16));
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.process_pty();

                self.renderer.update_screen(&self.screen);

                match self.renderer.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        if let Some(config) = &self.renderer.config {
                            self.renderer
                                .resize(PhysicalSize::new(config.width, config.height));
                        }
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        event_loop.exit();
                    }
                    Err(e) => {
                        warn!("Render error: {:?}", e);
                    }
                }

                self.renderer.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(&event);
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                use crate::input::{
                    MouseButton as InputMouseButton, MouseEvent as InputMouseEvent,
                };
                use mochi_core::screen::{MouseEncoding, MouseMode};
                use winit::event::{ElementState, MouseButton};

                let mouse_mode = self.screen.modes.mouse_tracking;
                let sgr_mode = matches!(self.screen.modes.mouse_encoding, MouseEncoding::Sgr);

                // Convert winit button to our input button
                let input_button = match button {
                    MouseButton::Left => Some(InputMouseButton::Left),
                    MouseButton::Middle => Some(InputMouseButton::Middle),
                    MouseButton::Right => Some(InputMouseButton::Right),
                    _ => None,
                };

                // If mouse tracking is enabled, send mouse events to PTY
                if !matches!(mouse_mode, MouseMode::None) {
                    if let Some(btn) = input_button {
                        let event = match state {
                            ElementState::Pressed => InputMouseEvent::Press(btn),
                            ElementState::Released => InputMouseEvent::Release(btn),
                        };
                        let bytes = self.input_encoder.encode_mouse(
                            event,
                            self.mouse_col,
                            self.mouse_row,
                            mouse_mode,
                            sgr_mode,
                        );
                        if !bytes.is_empty() {
                            if let Some(pty) = &self.pty {
                                let _ = pty.write(&bytes);
                            }
                        }
                    }
                }

                // Handle selection for left button when mouse tracking is disabled
                if button == MouseButton::Left && matches!(mouse_mode, MouseMode::None) {
                    match state {
                        ElementState::Pressed => {
                            self.mouse_pressed = true;
                            // Selection will be started on CursorMoved
                        }
                        ElementState::Released => {
                            self.mouse_pressed = false;
                            self.selection.end_selection();
                            // Copy selection to clipboard
                            if !self.selection.is_empty() {
                                let text = self.get_selection_text();
                                if let Some(clipboard) = &mut self.clipboard {
                                    let _ = clipboard.set_text(text);
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some((cell_width, cell_height)) = self.renderer.cell_dimensions() {
                    let col = (position.x / cell_width as f64) as usize;
                    let row = (position.y / cell_height as f64) as i64 - self.scroll_offset;

                    // Update tracked mouse position for mouse reporting
                    self.mouse_col = col.min(self.renderer.cols.saturating_sub(1)) as u16;
                    self.mouse_row =
                        row.clamp(0, self.renderer.rows.saturating_sub(1) as i64) as u16;

                    // Check if mouse tracking is enabled
                    use mochi_core::screen::{MouseEncoding, MouseMode};
                    let mouse_mode = self.screen.modes.mouse_tracking;

                    if matches!(mouse_mode, MouseMode::None) {
                        // Handle selection when mouse tracking is disabled
                        if self.mouse_pressed {
                            if self.selection.is_empty() || !self.selection.active {
                                // Start new selection
                                self.selection.start_selection(
                                    row,
                                    col.min(self.renderer.cols.saturating_sub(1)),
                                    mochi_core::SelectionType::Normal,
                                );
                            } else {
                                // Update existing selection
                                self.selection.update_selection(
                                    row,
                                    col.min(self.renderer.cols.saturating_sub(1)),
                                );
                            }
                        }
                    } else if matches!(mouse_mode, MouseMode::ButtonEvent | MouseMode::AnyEvent)
                        && self.mouse_pressed
                    {
                        // Send mouse motion events when tracking is enabled
                        use crate::input::MouseEvent as InputMouseEvent;
                        let sgr_mode =
                            matches!(self.screen.modes.mouse_encoding, MouseEncoding::Sgr);
                        let bytes = self.input_encoder.encode_mouse(
                            InputMouseEvent::Move,
                            self.mouse_col,
                            self.mouse_row,
                            mouse_mode,
                            sgr_mode,
                        );
                        if !bytes.is_empty() {
                            if let Some(pty) = &self.pty {
                                let _ = pty.write(&bytes);
                            }
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                use winit::event::MouseScrollDelta;
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as i64,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i64,
                };
                // Scroll up = positive offset (view older content)
                // Scroll down = negative offset (view newer content)
                let max_scroll = self.screen.scrollback().len() as i64;
                self.scroll_offset = (self.scroll_offset + lines).clamp(0, max_scroll);
            }
            _ => {}
        }
    }
}

impl TerminalApp {
    fn process_pty(&mut self) {
        if let Some(pty) = &self.pty {
            let mut buf = [0u8; 4096];
            loop {
                match pty.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        self.parser.parse(&buf[..n], |action| {
                            self.performer.perform(&mut self.screen, action);
                        });
                    }
                    Ok(_) => break,
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }

        // Sync bracketed paste mode from screen
        self.bracketed_paste_mode = self.screen.modes.bracketed_paste;

        // Sync application cursor keys mode
        self.input_encoder
            .set_application_cursor_keys(self.screen.modes.application_cursor_keys);
    }

    fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) {
        use crate::input::{Key as InputKey, Modifiers};
        use winit::event::ElementState;
        use winit::keyboard::{Key, ModifiersState, NamedKey};

        if event.state != ElementState::Pressed {
            return;
        }

        // Extract modifier state from the tracked modifiers
        let ctrl = self.modifiers.contains(ModifiersState::CONTROL);
        let shift = self.modifiers.contains(ModifiersState::SHIFT);
        let alt = self.modifiers.contains(ModifiersState::ALT);

        // Handle Ctrl+Shift+V for paste
        if ctrl && shift {
            if let Key::Character(c) = &event.logical_key {
                if c.eq_ignore_ascii_case("v") {
                    if let Some(clipboard) = &mut self.clipboard {
                        if let Ok(text) = clipboard.get_text() {
                            self.handle_paste(&text);
                        }
                    }
                    return;
                }
            }
        }

        let mods = Modifiers { shift, ctrl, alt };

        // Convert winit key to our InputKey
        let input_key = match &event.logical_key {
            Key::Character(c) => c.chars().next().map(InputKey::Char),
            Key::Named(named) => match named {
                NamedKey::Enter => Some(InputKey::Enter),
                NamedKey::Backspace => Some(InputKey::Backspace),
                NamedKey::Tab => Some(InputKey::Tab),
                NamedKey::Escape => Some(InputKey::Escape),
                NamedKey::ArrowUp => Some(InputKey::Up),
                NamedKey::ArrowDown => Some(InputKey::Down),
                NamedKey::ArrowRight => Some(InputKey::Right),
                NamedKey::ArrowLeft => Some(InputKey::Left),
                NamedKey::Home => Some(InputKey::Home),
                NamedKey::End => Some(InputKey::End),
                NamedKey::PageUp => Some(InputKey::PageUp),
                NamedKey::PageDown => Some(InputKey::PageDown),
                NamedKey::Insert => Some(InputKey::Insert),
                NamedKey::Delete => Some(InputKey::Delete),
                NamedKey::F1 => Some(InputKey::F(1)),
                NamedKey::F2 => Some(InputKey::F(2)),
                NamedKey::F3 => Some(InputKey::F(3)),
                NamedKey::F4 => Some(InputKey::F(4)),
                NamedKey::F5 => Some(InputKey::F(5)),
                NamedKey::F6 => Some(InputKey::F(6)),
                NamedKey::F7 => Some(InputKey::F(7)),
                NamedKey::F8 => Some(InputKey::F(8)),
                NamedKey::F9 => Some(InputKey::F(9)),
                NamedKey::F10 => Some(InputKey::F(10)),
                NamedKey::F11 => Some(InputKey::F(11)),
                NamedKey::F12 => Some(InputKey::F(12)),
                _ => None,
            },
            _ => None,
        };

        if let Some(key) = input_key {
            let bytes = self.input_encoder.encode_key(key, mods);
            if let Some(pty) = &self.pty {
                let _ = pty.write(&bytes);
            }
        }
    }

    /// Handle clipboard paste
    fn handle_paste(&mut self, text: &str) {
        let bytes = self
            .input_encoder
            .encode_paste(text, self.bracketed_paste_mode);
        if let Some(pty) = &self.pty {
            let _ = pty.write(&bytes);
        }
    }

    /// Get the text content of the current selection
    fn get_selection_text(&self) -> String {
        if self.selection.is_empty() {
            return String::new();
        }

        let (start, end) = self.selection.normalized();
        let mut result = String::new();

        for row in start.row..=end.row {
            if row < 0 || row >= self.screen.rows() as i64 {
                continue;
            }

            let Some(line) = self.screen.get_line(row as usize) else {
                continue;
            };
            let start_col = if row == start.row { start.col } else { 0 };
            let end_col = if row == end.row {
                end.col
            } else {
                self.screen.cols().saturating_sub(1)
            };

            for col in start_col..=end_col.min(self.screen.cols().saturating_sub(1)) {
                if let Some(cell) = line.get(col) {
                    if cell.character != '\0' && cell.character != ' ' || col > start_col {
                        result.push(cell.character);
                    }
                }
            }

            // Add newline between lines (but not after the last line)
            if row < end.row {
                result.push('\n');
            }
        }

        // Trim trailing whitespace from each line
        result
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Get foreground and background colors for a cell
fn get_cell_colors(cell: &mochi_core::Cell) -> ([f32; 4], [f32; 4]) {
    let fg = color_to_rgba(&cell.fg, true);
    let bg = color_to_rgba(&cell.bg, false);

    if cell.attrs.inverse {
        (bg, fg)
    } else {
        (fg, bg)
    }
}

/// Convert a Color to RGBA values
fn color_to_rgba(color: &Color, is_fg: bool) -> [f32; 4] {
    match color {
        Color::Default => {
            if is_fg {
                [0.9, 0.9, 0.9, 1.0]
            } else {
                [0.1, 0.1, 0.1, 1.0]
            }
        }
        Color::Indexed(idx) => indexed_color_to_rgba(*idx),
        Color::Rgb(r, g, b) => [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, 1.0],
    }
}

/// Convert an indexed color (0-255) to RGBA values
fn indexed_color_to_rgba(idx: u8) -> [f32; 4] {
    let colors: [[f32; 4]; 16] = [
        [0.0, 0.0, 0.0, 1.0],
        [0.8, 0.0, 0.0, 1.0],
        [0.0, 0.8, 0.0, 1.0],
        [0.8, 0.8, 0.0, 1.0],
        [0.0, 0.0, 0.8, 1.0],
        [0.8, 0.0, 0.8, 1.0],
        [0.0, 0.8, 0.8, 1.0],
        [0.75, 0.75, 0.75, 1.0],
        [0.5, 0.5, 0.5, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 0.0, 1.0, 1.0],
        [0.0, 1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0, 1.0],
    ];

    if idx < 16 {
        colors[idx as usize]
    } else if idx < 232 {
        let idx = idx - 16;
        let r = (idx / 36) % 6;
        let g = (idx / 6) % 6;
        let b = idx % 6;
        [
            if r > 0 {
                (r as f32 * 40.0 + 55.0) / 255.0
            } else {
                0.0
            },
            if g > 0 {
                (g as f32 * 40.0 + 55.0) / 255.0
            } else {
                0.0
            },
            if b > 0 {
                (b as f32 * 40.0 + 55.0) / 255.0
            } else {
                0.0
            },
            1.0,
        ]
    } else {
        let gray = ((idx - 232) as f32 * 10.0 + 8.0) / 255.0;
        [gray, gray, gray, 1.0]
    }
}

/// Create and run the terminal application
pub fn run_terminal(cols: usize, rows: usize) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;

    let mut pty = mochi_pty::Pty::new()?;
    pty.spawn(None, mochi_pty::PtySize::new(cols as u16, rows as u16))?;
    pty.set_nonblocking(true)?;

    info!("Shell spawned with PTY");

    // Initialize clipboard (may fail on headless systems)
    let clipboard = arboard::Clipboard::new().ok();
    if clipboard.is_none() {
        warn!("Failed to initialize clipboard - paste will not work");
    }

    let mut app = TerminalApp {
        renderer: Renderer::new(cols, rows),
        screen: Screen::new(cols, rows),
        parser: mochi_parser::Parser::new(),
        performer: crate::performer::Performer::new(),
        pty: Some(pty),
        input_encoder: crate::input::InputEncoder::new(),
        bracketed_paste_mode: false,
        modifiers: winit::keyboard::ModifiersState::empty(),
        clipboard,
        selection: mochi_core::Selection::new(),
        mouse_pressed: false,
        scroll_offset: 0,
        mouse_col: 0,
        mouse_row: 0,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
