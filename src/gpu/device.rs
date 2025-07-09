use wgpu;

pub struct GpuDevice {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuDevice {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            })
            .await?;

        Ok(GpuDevice { device, queue })
    }

    pub fn new_blocking() -> Result<Self, Box<dyn std::error::Error>> {
        pollster::block_on(Self::new())
    }
}
