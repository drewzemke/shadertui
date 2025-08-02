use std::time::Instant;
use wgpu;

use crate::gpu::{GpuDevice, UniformBuffer, Uniforms};
use crate::utils::{
    shader_shell::{get_window_display_shader, inject_user_shader, ShellType},
    threading::PerformanceTracker,
};

// AIDEV-NOTE: WindowRenderer uses compute+render pipeline: compute shader writes to texture, fragment shader displays it
pub struct WindowRenderer {
    surface: wgpu::Surface<'static>,

    // Compute stage: user's shader writes to storage texture
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    uniform_buffer: UniformBuffer,

    // Render stage: simple fragment shader samples from storage texture
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,

    gpu_device: GpuDevice,
    adapter: wgpu::Adapter,
    width: u32,
    height: u32,

    // Time tracking for uniform updates
    start_time: Instant,
    last_frame_time: Instant,
    frame_count: u32,

    // Input state
    cursor_position: [f32; 2],
    is_paused: bool,
    paused_time: f32,

    // Performance tracking
    performance_tracker: Option<PerformanceTracker>,
}

impl WindowRenderer {
    pub fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window_size: (u32, u32),
        shader_source: &str,
        enable_performance_tracking: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Get adapter compatible with the surface
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))?;

        // Create device and queue
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            }))?;

        let gpu_device = GpuDevice { device, queue };

        // Use provided window size
        let width = window_size.0;
        let height = window_size.1;

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
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&gpu_device.device, &surface_config);

        // Initialize time tracking
        let now = Instant::now();

        // Create uniform buffer - will be updated each frame
        let uniform_buffer = UniformBuffer::new(&gpu_device.device);
        let uniforms = Uniforms {
            resolution: [width as f32, height as f32],
            cursor: [0.0, 0.0],
            time: 0.0,
            frame: 0,
            delta_time: 0.0,
            _padding: 0.0,
        };
        uniform_buffer.update(&gpu_device.queue, &uniforms);

        // Create storage texture for compute shader output
        let storage_texture = gpu_device.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Storage Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let storage_texture_view =
            storage_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create compute pipeline with injected user shader in window shell
        let complete_shader = inject_user_shader(shader_source, ShellType::Window)?;
        let (compute_pipeline, compute_bind_group_layout) =
            Self::create_compute_pipeline(&gpu_device.device, &complete_shader)?;

        // Create compute bind group
        let compute_bind_group = gpu_device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compute Bind Group"),
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&storage_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform_buffer.buffer.as_entire_binding(),
                    },
                ],
            });

        // Create sampler for texture sampling
        let sampler = gpu_device.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Storage Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create render pipeline (samples from storage texture)
        let (render_pipeline, render_bind_group) = Self::create_render_pipeline(
            &gpu_device.device,
            &storage_texture_view,
            &sampler,
            surface_format,
        )?;

        Ok(Self {
            surface,
            compute_pipeline,
            compute_bind_group,
            uniform_buffer,
            render_pipeline,
            render_bind_group,
            gpu_device,
            adapter,
            width,
            height,
            start_time: now,
            last_frame_time: now,
            frame_count: 0,
            cursor_position: [0.0, 0.0],
            is_paused: false,
            paused_time: 0.0,
            performance_tracker: if enable_performance_tracking {
                Some(PerformanceTracker::new())
            } else {
                None
            },
        })
    }

    fn create_compute_pipeline(
        device: &wgpu::Device,
        shader_source: &str,
    ) -> Result<(wgpu::ComputePipeline, wgpu::BindGroupLayout), Box<dyn std::error::Error>> {
        // Create shader module
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[
                // Storage texture for output
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Uniform buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok((pipeline, bind_group_layout))
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        storage_texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        surface_format: wgpu::TextureFormat,
    ) -> Result<(wgpu::RenderPipeline, wgpu::BindGroup), Box<dyn std::error::Error>> {
        // Use the window display shader from template file
        let shader_source = get_window_display_shader();

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(storage_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        // Create render pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
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
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
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

        Ok((render_pipeline, bind_group))
    }

    // AIDEV-NOTE: Public methods for controlling renderer state from event loop
    pub fn update_cursor_position(&mut self, x: f32, y: f32) {
        // Store cursor in pixel coordinates, flipping Y axis (window Y=0 at top, shader Y=0 at bottom)
        self.cursor_position = [x, self.height as f32 - y];
        println!(
            "Updated cursor: ({x:.3}, {y:.3}) -> flipped: ({:.3}, {:.3})",
            self.cursor_position[0], self.cursor_position[1]
        );
    }

    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            // Resume: adjust start time to account for pause duration
            let pause_duration = Instant::now().duration_since(self.last_frame_time);
            self.start_time += pause_duration;
            self.is_paused = false;
        } else {
            // Pause: store current time
            self.paused_time = Instant::now().duration_since(self.start_time).as_secs_f32();
            self.is_paused = true;
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.width = width;
        self.height = height;

        // Reconfigure surface using stored adapter
        let surface_caps = self.surface.get_capabilities(&self.adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        self.surface
            .configure(&self.gpu_device.device, &surface_config);

        // Recreate storage texture with new size
        let storage_texture = self
            .gpu_device
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Storage Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

        let storage_texture_view =
            storage_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Update compute bind group with new texture
        self.compute_bind_group =
            self.gpu_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Compute Bind Group"),
                    layout: &self.compute_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&storage_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.uniform_buffer.buffer.as_entire_binding(),
                        },
                    ],
                });

        // Update render bind group with new texture
        let sampler = self
            .gpu_device
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Storage Texture Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

        self.render_bind_group =
            self.gpu_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Render Bind Group"),
                    layout: &self.render_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&storage_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                });

        Ok(())
    }

    // AIDEV-NOTE: Performance tracking methods for window title display
    pub fn get_fps(&self) -> Option<f32> {
        self.performance_tracker
            .as_ref()
            .map(|tracker| tracker.get_fps())
    }

    // AIDEV-NOTE: Hot reload method for shader recompilation
    pub fn reload_shader(
        &mut self,
        user_shader_source: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create new compute pipeline with injected user shader in window shell
        let complete_shader = inject_user_shader(user_shader_source, ShellType::Window)?;
        let (new_compute_pipeline, compute_bind_group_layout) =
            Self::create_compute_pipeline(&self.gpu_device.device, &complete_shader)?;

        // Update compute pipeline
        self.compute_pipeline = new_compute_pipeline;

        // Recreate compute bind group with new pipeline layout
        let storage_texture = self
            .gpu_device
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Storage Texture"),
                size: wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

        let storage_texture_view =
            storage_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Update compute bind group
        self.compute_bind_group =
            self.gpu_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Compute Bind Group"),
                    layout: &compute_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&storage_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.uniform_buffer.buffer.as_entire_binding(),
                        },
                    ],
                });

        // Update render bind group with new texture
        let sampler = self
            .gpu_device
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Storage Texture Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

        self.render_bind_group =
            self.gpu_device
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Render Bind Group"),
                    layout: &self.render_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&storage_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                });

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update time and uniforms
        let current_time = Instant::now();
        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;

        let time = if self.is_paused {
            self.paused_time
        } else {
            current_time.duration_since(self.start_time).as_secs_f32()
        };

        self.frame_count += 1;

        // Update uniform buffer
        let uniforms = Uniforms {
            resolution: [self.width as f32, self.height as f32],
            cursor: self.cursor_position,
            time,
            frame: self.frame_count,
            delta_time,
            _padding: 0.0,
        };
        self.uniform_buffer
            .update(&self.gpu_device.queue, &uniforms);
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.gpu_device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Window Render Encoder"),
                });

        // Stage 1: Compute pass - run user's shader to generate output texture
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            // Dispatch compute shader with 8x8 workgroup size
            let workgroup_count_x = self.width.div_ceil(8);
            let workgroup_count_y = self.height.div_ceil(8);
            compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
        }

        // Stage 2: Render pass - sample from storage texture and present to surface
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Draw fullscreen triangle
        }

        self.gpu_device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        // Record frame for performance tracking
        if let Some(ref mut tracker) = self.performance_tracker {
            tracker.record_frame();
        }

        Ok(())
    }
}
