use std::sync::Arc;
use wgpu;

use crate::gpu::{GpuDevice, UniformBuffer, Uniforms};
use crate::utils::threading::PerformanceTracker;

use super::window::{GpuResourceManager, PipelineFactory, SurfaceManager, WindowState};

// AIDEV-NOTE: WindowRenderer uses compute+render pipeline: compute shader writes to texture, fragment shader displays it
pub struct WindowRenderer {
    surface_manager: SurfaceManager,
    resource_manager: GpuResourceManager,

    // Compute stage: user's shader writes to storage texture
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: UniformBuffer,

    // Render stage: simple fragment shader samples from storage texture
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    render_bind_group_layout: wgpu::BindGroupLayout,

    gpu_device: GpuDevice,
    state: WindowState,
    width: u32,
    height: u32,

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
        let width = window_size.0;
        let height = window_size.1;

        // Initialize utility managers
        let surface_manager = SurfaceManager::new(surface, adapter);
        let resource_manager = GpuResourceManager::new(Arc::new(gpu_device.device.clone()));

        // Configure surface
        let surface_format = surface_manager.get_optimal_format();
        surface_manager.configure(&gpu_device.device, width, height);

        // Create uniform buffer
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

        // Create GPU resources
        let storage_texture = resource_manager.create_storage_texture(width, height);
        let storage_texture_view =
            storage_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = resource_manager.create_sampler();

        // Create pipelines
        let (compute_pipeline, compute_bind_group_layout) =
            PipelineFactory::create_compute_pipeline_with_user_shader(
                &gpu_device.device,
                shader_source,
            )?;
        let (render_pipeline, render_bind_group_layout) =
            PipelineFactory::create_render_pipeline(&gpu_device.device, surface_format)?;

        // Create bind groups
        let compute_bind_group = resource_manager.create_compute_bind_group(
            &compute_bind_group_layout,
            &storage_texture_view,
            &uniform_buffer,
        );
        let render_bind_group = resource_manager.create_render_bind_group(
            &render_bind_group_layout,
            &storage_texture_view,
            &sampler,
        );

        Ok(Self {
            surface_manager,
            resource_manager,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            uniform_buffer,
            render_pipeline,
            render_bind_group,
            render_bind_group_layout,
            gpu_device,
            state: WindowState::new(),
            width,
            height,
            performance_tracker: if enable_performance_tracking {
                Some(PerformanceTracker::new())
            } else {
                None
            },
        })
    }

    // AIDEV-NOTE: Public methods for controlling renderer state from event loop
    pub fn update_cursor_position(&mut self, x: f32, y: f32) {
        self.state.update_cursor_position(x, y, self.height);
    }

    pub fn toggle_pause(&mut self) {
        self.state.toggle_pause();
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.width = width;
        self.height = height;

        // Reconfigure surface
        self.surface_manager
            .configure(&self.gpu_device.device, width, height);

        // Recreate GPU resources with new size
        let storage_texture = self.resource_manager.create_storage_texture(width, height);
        let storage_texture_view =
            storage_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.resource_manager.create_sampler();

        // Update bind groups with new texture
        self.compute_bind_group = self.resource_manager.create_compute_bind_group(
            &self.compute_bind_group_layout,
            &storage_texture_view,
            &self.uniform_buffer,
        );
        self.render_bind_group = self.resource_manager.create_render_bind_group(
            &self.render_bind_group_layout,
            &storage_texture_view,
            &sampler,
        );

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
        // Create new compute pipeline with injected user shader
        let (new_compute_pipeline, new_compute_bind_group_layout) =
            PipelineFactory::create_compute_pipeline_with_user_shader(
                &self.gpu_device.device,
                user_shader_source,
            )?;

        // Update compute pipeline and layout
        self.compute_pipeline = new_compute_pipeline;
        self.compute_bind_group_layout = new_compute_bind_group_layout;

        // Recreate GPU resources
        let storage_texture = self
            .resource_manager
            .create_storage_texture(self.width, self.height);
        let storage_texture_view =
            storage_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.resource_manager.create_sampler();

        // Update bind groups with new resources
        self.compute_bind_group = self.resource_manager.create_compute_bind_group(
            &self.compute_bind_group_layout,
            &storage_texture_view,
            &self.uniform_buffer,
        );
        self.render_bind_group = self.resource_manager.create_render_bind_group(
            &self.render_bind_group_layout,
            &storage_texture_view,
            &sampler,
        );

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update time and uniforms using state manager
        let delta_time = self.state.update_frame_timing();
        let time = self.state.get_current_time();

        // Update uniform buffer
        let uniforms = Uniforms {
            resolution: [self.width as f32, self.height as f32],
            cursor: self.state.cursor_position,
            time,
            frame: self.state.frame_count,
            delta_time,
            _padding: 0.0,
        };
        self.uniform_buffer
            .update(&self.gpu_device.queue, &uniforms);

        let output = self.surface_manager.get_current_texture()?;
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
