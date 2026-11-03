//! Offscreen validation fixture. Never opens musicOS or its user profile.
use musicos_visualizer::{catalog, gpu::Gpu, model::Audio, renderer::Layer};
use std::{path::PathBuf, time::Instant};
fn main() -> anyhow::Result<()> {
    let folder = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "out/visualizer-reference/rust".into()),
    );
    std::fs::create_dir_all(&folder)?;
    let gpu = pollster::block_on(Gpu::new(None, Gpu::instance()))?;
    eprintln!("Adapter: {:?}", gpu.adapter.get_info());
    let frames = std::env::var("FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);
    for preset in catalog::catalog() {
        let reference = std::env::var("REFERENCE_AUDIO")
            .ok()
            .map(|folder| {
                std::fs::read_to_string(
                    std::path::Path::new(&folder).join(format!("{}.json", preset.id)),
                )
            })
            .transpose()?
            .map(|s| serde_json::from_str::<serde_json::Value>(&s))
            .transpose()?;
        let begin = Instant::now();
        gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let state = reference
            .as_ref()
            .and_then(|v| serde_json::from_value::<[u32; 4]>(v["initialRandomState"].clone()).ok());
        let mut layer = Layer::with_reference_state(&gpu, &preset.id, 640, 480, 42, state)?;
        let mut audio = Audio::default();
        for frame in 0..frames {
            let t = frame as f32 / 60.;
            for c in 0..2 {
                for i in 0..512 {
                    let phase = t * 48000. + i as f32 * 2.;
                    audio.waveform[c][i] =
                        (phase * std::f32::consts::TAU * (110. + c as f32 * 55.) / 48000.).sin()
                            * 0.4;
                    audio.spectrum[c][i] = 0.2;
                }
            }
            audio.bands = [1.0 + (t * 4.).sin() as f64 * 0.4, 1., 1.];
            audio.attenuated = [1.; 3];
            if let Some(reference) = &reference {
                let f = &reference["features"][frame];
                for c in 0..2 {
                    for i in 0..512 {
                        audio.waveform[c][i] = f["waveform"][c][i].as_f64().unwrap_or(0.) as f32;
                        audio.spectrum[c][i] = f["spectrum"][c][i].as_f64().unwrap_or(0.) as f32;
                    }
                }
                for i in 0..3 {
                    audio.bands[i] = f["bands"][i].as_f64().unwrap();
                    audio.attenuated[i] = f["attenuated"][i].as_f64().unwrap();
                }
            }
            layer.render(&gpu, 1. / 60., &audio)?;
        }
        if let Some(reference) = &reference {
            let mut model = reference["values"][0]["frame"]
                .as_object()
                .unwrap()
                .keys()
                .map(|key| (key.clone(), serde_json::json!(layer.model.get(key))))
                .collect::<serde_json::Map<_, _>>();
            model.insert(
                "_uv".into(),
                serde_json::json!(layer
                    .model
                    .vertices
                    .iter()
                    .flat_map(|v| v.uv)
                    .collect::<Vec<_>>()),
            );
            std::fs::write(
                folder.join(format!("{}-values.json", preset.id)),
                serde_json::to_vec(&model)?,
            )?;
        }
        let image = layer.output();
        let row = 640 * 4;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Offscreen validation readback"),
            size: (row * 480) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &image.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(480),
                },
            },
            image.texture.size(),
        );
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
        let mapped = buffer.slice(..).get_mapped_range();
        let mut ppm = format!("P6\n640 480\n255\n").into_bytes();
        for pixel in mapped.chunks_exact(4) {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(folder.join(format!("{}.ppm", preset.id)), ppm)?;
        eprintln!(
            "{}: {} frames, {:.2} ms/frame including initialization and readback",
            preset.id,
            frames,
            begin.elapsed().as_secs_f64() * 1000. / frames as f64
        );
    }
    Ok(())
}
