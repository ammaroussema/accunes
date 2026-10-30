use winit::window::Window;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FilterParams {
    pub filter_mode: u32, // 0 = none, 1 = scanlines, 2 = lcd_grid
    pub scanline_scale: u32,
    pub cell_w: f32,
    pub cell_h: f32,
}

pub struct GpuRenderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    nes_pipeline: wgpu::RenderPipeline,
    ui_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler_nearest: wgpu::Sampler,
    sampler_linear: wgpu::Sampler,

    filter_buffer: wgpu::Buffer,
    supported_present_modes: Vec<wgpu::PresentMode>,

    nes_texture: Option<wgpu::Texture>,
    nes_bind_group: Option<wgpu::BindGroup>,
    nes_width: u32,
    nes_height: u32,
    nes_linear: bool,

    ui_texture: Option<wgpu::Texture>,
    ui_bind_group: Option<wgpu::BindGroup>,
    ui_width: u32,
    ui_height: u32,
}

const SHADER_SOURCE: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct FilterParams {
    filter_mode: u32,
    scanline_scale: u32,
    cell_w: f32,
    cell_h: f32,
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> params: FilterParams;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((in_vertex_index << 1u) & 2u);
    let y = f32(in_vertex_index & 2u);
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.tex_coords = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    if (params.filter_mode == 1u) {
        let line = u32(in.position.y);
        if (params.scanline_scale > 1u && (line % params.scanline_scale) != 0u) {
            color = vec4<f32>(color.rgb * (187.0 / 256.0), color.a);
        }
    } else if (params.filter_mode == 2u) {
        let fx = fract(in.position.x / params.cell_w);
        let fy = fract(in.position.y / params.cell_h);
        var m = 1.0;
        if (fx < 0.5 && fy < 0.5) {
            m = 1.0;
        } else if (fx >= 0.5 && fy >= 0.5) {
            m = 0.35;
        } else {
            m = 0.45;
        }
        color = vec4<f32>(color.rgb * m, color.a);
    }

    return color;
}
"#;

impl GpuRenderer {
    pub fn new(window: &Window) -> Result<Self, String> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(window) }
            .map_err(|e| format!("Failed to create surface: {:?}", e))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "Failed to find suitable GPU adapter".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: Some("accunes_device"),
            },
            None,
        ))
        .map_err(|e| format!("Failed to create device: {:?}", e))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let supported_present_modes = surface_caps.present_modes.clone();
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = if surface_caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            surface_caps.present_modes[0]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let filter_params = FilterParams {
            filter_mode: 0,
            scanline_scale: 1,
            cell_w: 1.0,
            cell_h: 1.0,
        };
        let filter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter_params_buffer"),
            size: std::mem::size_of::<FilterParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&filter_buffer, 0, bytemuck::bytes_of(&filter_params));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("present_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("present_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let nes_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nes_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
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
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler_nearest"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler_linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            nes_pipeline,
            ui_pipeline,
            bind_group_layout,
            sampler_nearest,
            sampler_linear,
            filter_buffer,
            supported_present_modes,
            nes_texture: None,
            nes_bind_group: None,
            nes_width: 0,
            nes_height: 0,
            nes_linear: false,
            ui_texture: None,
            ui_bind_group: None,
            ui_width: 0,
            ui_height: 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.ui_texture = None;
            self.ui_bind_group = None;
            self.ui_width = 0;
            self.ui_height = 0;
        }
    }

    pub fn set_fast_forward(&mut self, fast: bool) {
        let desired_mode = if fast {
            if self.supported_present_modes.contains(&wgpu::PresentMode::Immediate) {
                wgpu::PresentMode::Immediate
            } else if self.supported_present_modes.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else {
                wgpu::PresentMode::Fifo
            }
        } else {
            if self.supported_present_modes.contains(&wgpu::PresentMode::Fifo) {
                wgpu::PresentMode::Fifo
            } else {
                self.supported_present_modes[0]
            }
        };

        if self.config.present_mode != desired_mode {
            self.config.present_mode = desired_mode;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn set_filter_params(&mut self, params: FilterParams) {
        self.queue.write_buffer(&self.filter_buffer, 0, bytemuck::bytes_of(&params));
    }

    pub fn update_nes_frame(&mut self, width: u32, height: u32, pixels: &[u32], linear_filter: bool) {
        if width == 0 || height == 0 || pixels.is_empty() {
            return;
        }
        if self.nes_width != width || self.nes_height != height || self.nes_linear != linear_filter || self.nes_texture.is_none() {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("nes_frame_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = if linear_filter {
                &self.sampler_linear
            } else {
                &self.sampler_nearest
            };
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("nes_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.filter_buffer.as_entire_binding(),
                    },
                ],
            });
            self.nes_texture = Some(texture);
            self.nes_bind_group = Some(bind_group);
            self.nes_width = width;
            self.nes_height = height;
            self.nes_linear = linear_filter;
        }

        if let Some(texture) = &self.nes_texture {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(pixels),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub fn update_ui_frame(&mut self, width: u32, height: u32, pixels: &[u32]) {
        if width == 0 || height == 0 || pixels.is_empty() {
            return;
        }
        if self.ui_width != width || self.ui_height != height || self.ui_texture.is_none() {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ui_overlay_texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ui_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler_nearest),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.filter_buffer.as_entire_binding(),
                    },
                ],
            });
            self.ui_texture = Some(texture);
            self.ui_bind_group = Some(bind_group);
            self.ui_width = width;
            self.ui_height = height;
        }

        if let Some(texture) = &self.ui_texture {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(pixels),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub fn render(
        &mut self,
        dest_rect: Option<(u32, u32, u32, u32)>,
        bg_color_u32: u32,
        ui_active: bool,
    ) -> Result<(), wgpu::SurfaceError> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => {
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("present_command_encoder"),
        });

        let r = (((bg_color_u32 >> 16) & 0xFF) as f64) / 255.0;
        let g = (((bg_color_u32 >> 8) & 0xFF) as f64) / 255.0;
        let b = ((bg_color_u32 & 0xFF) as f64) / 255.0;

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screen_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            if let (Some(bind_group), Some((dx, dy, dw, dh))) = (&self.nes_bind_group, dest_rect) {
                if dw > 0 && dh > 0 {
                    pass.set_pipeline(&self.nes_pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_viewport(dx as f32, dy as f32, dw as f32, dh as f32, 0.0, 1.0);
                    let scissor_w = dw.min(self.config.width.saturating_sub(dx));
                    let scissor_h = dh.min(self.config.height.saturating_sub(dy));
                    if scissor_w > 0 && scissor_h > 0 {
                        pass.set_scissor_rect(dx, dy, scissor_w, scissor_h);
                        pass.draw(0..3, 0..1);
                    }
                }
            }

            if ui_active {
                if let Some(bind_group) = &self.ui_bind_group {
                    pass.set_pipeline(&self.ui_pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_viewport(0.0, 0.0, self.config.width as f32, self.config.height as f32, 0.0, 1.0);
                    pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                    pass.draw(0..3, 0..1);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}
