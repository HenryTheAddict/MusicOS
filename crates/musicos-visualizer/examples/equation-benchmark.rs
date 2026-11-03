//! Like-for-like main-frame + pixel-mesh CPU timing, without GPU submission.
use musicos_visualizer::{
    catalog,
    model::{Audio, Model},
};
use std::time::Instant;
fn main() -> anyhow::Result<()> {
    for p in catalog::catalog() {
        let reference: serde_json::Value = serde_json::from_slice(&std::fs::read(format!(
            "out/visualizer-reference/butterchurn-renders/{}.json",
            p.id
        ))?)?;
        let audio = reference["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                let mut a = Audio::default();
                for c in 0..2 {
                    for i in 0..512 {
                        a.waveform[c][i] = f["waveform"][c][i].as_f64().unwrap_or(0.) as f32;
                        a.spectrum[c][i] = f["spectrum"][c][i].as_f64().unwrap_or(0.) as f32;
                    }
                }
                for i in 0..3 {
                    a.bands[i] = f["bands"][i].as_f64().unwrap();
                    a.attenuated[i] = f["attenuated"][i].as_f64().unwrap();
                }
                a
            })
            .collect::<Vec<_>>();
        let mut times = Vec::new();
        for _ in 0..5 {
            let mut m = Model::load(&p.id, 42)?;
            for f in &audio {
                let now = Instant::now();
                m.advance(1. / 60., 640, 480, f);
                std::hint::black_box(&m.vertices);
                times.push(now.elapsed().as_secs_f64() * 1000.);
            }
        }
        times.sort_by(f64::total_cmp);
        println!(
            "{} mean {:.4}ms p95 {:.4}ms",
            p.id,
            times.iter().sum::<f64>() / times.len() as f64,
            times[times.len() * 95 / 100]
        );
    }
    Ok(())
}
