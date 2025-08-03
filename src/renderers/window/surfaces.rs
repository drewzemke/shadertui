use wgpu;

// AIDEV-NOTE: Extracted surface management from WindowRenderer to eliminate surface config duplication
pub struct SurfaceManager {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
}

impl SurfaceManager {
    pub fn new(surface: wgpu::Surface<'static>, adapter: wgpu::Adapter) -> Self {
        Self { surface, adapter }
    }

    pub fn configure(&self, device: &wgpu::Device, width: u32, height: u32) {
        let surface_config = self.create_surface_config(width, height);
        self.surface.configure(device, &surface_config);
    }

    pub fn get_optimal_format(&self) -> wgpu::TextureFormat {
        let surface_caps = self.surface.get_capabilities(&self.adapter);
        surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0])
    }

    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    fn create_surface_config(&self, width: u32, height: u32) -> wgpu::SurfaceConfiguration {
        let surface_caps = self.surface.get_capabilities(&self.adapter);
        let surface_format = self.get_optimal_format();

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }
}
