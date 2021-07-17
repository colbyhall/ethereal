use crate::{
    engine,
    asset,

    Instance,
    Device,
    RenderPass,
    Pipeline,
    Format,
};

use engine::{ Engine, Module, Builder };
use asset::AssetVariant;

pub struct Gpu {
    device: Device,
    backbuffer_render_pass: RenderPass,
}

impl Gpu {
    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn backbuffer_render_pass(&self) -> &RenderPass {
        &self.backbuffer_render_pass
    }
}

impl Module for Gpu {
    fn new() -> Self { 
        let engine = Engine::as_ref();

        let instance = Instance::new().unwrap();
        let device = instance.create_device(Some(engine.window().handle())).unwrap();

        let backbuffer_render_pass = device.create_render_pass(vec![Format::BGR_U8_SRGB], None).unwrap();

        Self { device, backbuffer_render_pass }
     }

    fn depends_on(builder: Builder) -> Builder {
        builder
            .register(AssetVariant::new::<Pipeline>(&["pipeline"]))
    }
}