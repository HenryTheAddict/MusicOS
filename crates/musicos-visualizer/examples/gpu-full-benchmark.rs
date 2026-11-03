//! Complete preset CPU submission and Metal execution, without app/audio output.
use musicos_visualizer::{catalog, gpu::Gpu, model::Audio, renderer::Layer};
use std::time::Instant;
fn main() -> anyhow::Result<()> {
    let gpu = pollster::block_on(Gpu::new(None, Gpu::instance()))?;
    let queries = gpu
        .device
        .features()
        .contains(wgpu::Features::TIMESTAMP_QUERY);
    for p in catalog::catalog() {
        let reference: serde_json::Value = serde_json::from_slice(&std::fs::read(format!(
            "out/visualizer-reference/butterchurn-renders/{}.json",
            p.id
        ))?)?;
        let features = reference["features"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing reference audio"))?;
        let state = serde_json::from_value(reference["initialRandomState"].clone()).ok();
        let mut layer = Layer::with_reference_state(&gpu, &p.id, 640, 480, 42, state)?;
        let query = queries.then(|| {
            gpu.device.create_query_set(&wgpu::QuerySetDescriptor {
                label: None,
                ty: wgpu::QueryType::Timestamp,
                count: 360,
            })
        });
        let mut times = Vec::new();
        for frame in 0..180 {
            let f = &features[frame as usize];
            let mut audio = Audio::default();
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
            let now = Instant::now();
            layer.render_profiled(
                &gpu,
                1. / 60.,
                &audio,
                query.as_ref().map(|q| (q, frame * 2, frame * 2 + 1)),
            )?;
            times.push(now.elapsed().as_secs_f64() * 1000.);
        }
        let mut gpu_mean = None;
        if let Some(query) = query {
            let resolved = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 360 * 8,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 360 * 8,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.resolve_query_set(&query, 0..360, &resolved, 0);
            encoder.copy_buffer_to_buffer(&resolved, 0, &read, 0, read.size());
            gpu.queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            gpu.device.poll(wgpu::PollType::Wait)?;
            rx.recv()??;
            let data = read.slice(..).get_mapped_range();
            let values = bytemuck::cast_slice::<u8, u64>(&data);
            gpu_mean = Some(
                values
                    .chunks_exact(2)
                    .skip(10)
                    .map(|v| {
                        v[1].saturating_sub(v[0]) as f64 * gpu.queue.get_timestamp_period() as f64
                            / 1e6
                    })
                    .sum::<f64>()
                    / 170.,
            );
        }
        times.drain(..10);
        times.sort_by(f64::total_cmp);
        println!(
            "{} full CPU mean {:.4}ms p95 {:.4}ms; Metal full preset mean {:?}ms",
            p.id,
            times.iter().sum::<f64>() / times.len() as f64,
            times[times.len() * 95 / 100],
            gpu_mean
        );
    }
    Ok(())
}
