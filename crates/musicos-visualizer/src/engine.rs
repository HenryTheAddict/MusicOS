//! One GPU owner, with at most the active and outgoing preset. Interrupted
//! blends retain the displayed image rather than accumulating simulations.
use crate::{
    gpu::Gpu,
    model::Audio,
    renderer::{Image, Layer, Presenter},
};
use anyhow::Result;
enum Outgoing {
    Live(Layer),
    Frozen(Image),
}
impl Outgoing {
    fn image(&self) -> &Image {
        match self {
            Self::Live(layer) => layer.output(),
            Self::Frozen(image) => image,
        }
    }
}
pub struct Engine {
    active: Option<Layer>,
    outgoing: Option<Outgoing>,
    composite: Image,
    presenter: Presenter,
    blend: f64,
    duration: f64,
    seed: u64,
    pub rendered: u64,
}
impl Engine {
    pub fn new(gpu: &Gpu, id: &str, width: u32, height: u32, seed: u64) -> Result<Self> {
        Ok(Self {
            active: Some(Layer::new(gpu, id, width, height, seed)?),
            outgoing: None,
            composite: Image::new(&gpu.device, width, height),
            presenter: Presenter::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm),
            blend: 0.,
            duration: 0.,
            seed,
            rendered: 0,
        })
    }
    pub fn preset(&self) -> &str {
        &self.active.as_ref().unwrap().model.id
    }
    pub fn select(&mut self, gpu: &Gpu, id: &str, duration: f64) -> Result<()> {
        if self.preset() == id {
            return Ok(());
        }
        let incoming = Layer::new(
            gpu,
            id,
            self.composite.width,
            self.composite.height,
            self.seed.wrapping_add(self.rendered),
        )?;
        if duration <= 0. {
            self.active = Some(incoming);
            self.outgoing = None;
            self.duration = 0.;
            return Ok(());
        }
        self.outgoing = if self.outgoing.is_some() {
            let frozen = Image::new(&gpu.device, self.composite.width, self.composite.height);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_texture(
                self.composite.texture.as_image_copy(),
                frozen.texture.as_image_copy(),
                self.composite.texture.size(),
            );
            gpu.queue.submit([encoder.finish()]);
            Some(Outgoing::Frozen(frozen))
        } else {
            self.active.take().map(Outgoing::Live)
        };
        self.active = Some(incoming);
        self.blend = 0.;
        self.duration = duration.clamp(0.5, 3.);
        Ok(())
    }
    pub fn transitioning(&self) -> bool {
        self.outgoing.is_some()
    }
    pub fn render(&mut self, gpu: &Gpu, dt: f64, audio: &Audio, accent: f32) -> Result<()> {
        self.render_timed(gpu, dt, dt, audio, accent)
    }
    pub fn render_timed(
        &mut self,
        gpu: &Gpu,
        dt: f64,
        blend_dt: f64,
        audio: &Audio,
        accent: f32,
    ) -> Result<()> {
        let layer = self.active.as_mut().unwrap();
        if dt > 0. || layer.model.frames == 0 {
            layer.render(gpu, dt, audio)?;
        }
        if let Some(Outgoing::Live(old)) = &mut self.outgoing {
            if dt > 0. {
                old.render(gpu, dt, audio)?;
            }
        }
        self.blend = (self.blend + blend_dt.max(0.)).min(self.duration);
        let t = if self.duration > 0. {
            (self.blend / self.duration) as f32
        } else {
            1.
        };
        self.presenter.draw(
            gpu,
            &self.composite.view,
            self.outgoing
                .as_ref()
                .map(|o| o.image())
                .unwrap_or(layer.output()),
            layer.output(),
            t,
            1. + accent,
        );
        if t >= 1. {
            self.outgoing = None;
        }
        self.rendered += 1;
        Ok(())
    }
    pub fn image(&self) -> &Image {
        &self.composite
    }
    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        if self.composite.width == width && self.composite.height == height {
            return;
        }
        if let Some(layer) = &mut self.active {
            layer.resize(gpu, width, height);
        }
        if let Some(Outgoing::Live(old)) = &mut self.outgoing {
            old.resize(gpu, width, height);
        }
        let target = Image::new(&gpu.device, width, height);
        self.presenter
            .draw(gpu, &target.view, &self.composite, &self.composite, 0., 1.);
        self.composite = target;
    }
}
/// Internal resolution keeps the window aspect ratio and never exceeds 1080p.
pub fn internal_size(width: u32, height: u32, quality: f64) -> (u32, u32) {
    let w = width.max(1) as f64;
    let h = height.max(1) as f64;
    let scale = (1920. / w).min(1080. / h).min(1.) * quality.clamp(0.4, 1.);
    (
        (w * scale).round().max(1.) as u32,
        (h * scale).round().max(1.) as u32,
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn aspect_and_cap_are_preserved() {
        assert_eq!(super::internal_size(3840, 2160, 1.), (1920, 1080));
        assert_eq!(super::internal_size(480, 1280, 1.), (405, 1080));
        assert_eq!(super::internal_size(480, 480, 1.), (480, 480));
    }
}
