//! GPU result vs f64 CPU equation reference; no app, audio output, or profile.
use musicos_visualizer::{catalog, gpu::Gpu, model::Audio, renderer::Layer};
fn main() -> anyhow::Result<()> {
    let gpu = pollster::block_on(Gpu::new(None, Gpu::instance()))?;
    for size in [(640, 480), (480, 1280), (1280, 800)] {
        for preset in catalog::catalog() {
            gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let mut layer = Layer::new(&gpu, &preset.id, size.0, size.1, 42)?;
            let mut audio = Audio::default();
            audio.bands = [1.2, 0.8, 1.4];
            for _ in 0..3 {
                layer.render(&gpu, 1. / 60., &audio)?;
            }
            layer.model.reference_mesh(size.0, size.1);
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GPU validation readback"),
                size: layer.mesh_buffer().size(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(layer.mesh_buffer(), 0, &buffer, 0, buffer.size());
            gpu.queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            gpu.device.poll(wgpu::PollType::Wait)?;
            rx.recv()??;
            if let Some(error) = pollster::block_on(gpu.device.pop_error_scope()) {
                anyhow::bail!("{}: {error}", preset.id);
            }
            let data = buffer.slice(..).get_mapped_range();
            let values = bytemuck::cast_slice::<u8, f32>(&data);
            let mut max = 0_f32;
            for (i, vertex) in layer.model.vertices.iter().enumerate() {
                for c in 0..2 {
                    let actual = values[i * 10 + 2 + c];
                    anyhow::ensure!(actual.is_finite(), "{} nonfinite", preset.id);
                    max = max.max((actual - vertex.uv[c]).abs());
                }
            }
            println!("{} {}x{} UV max error {max:.7}", preset.id, size.0, size.1);
            anyhow::ensure!(
                max < 0.002,
                "{} GPU equations diverged from CPU reference",
                preset.id
            );
        }
    }
    Ok(())
}
