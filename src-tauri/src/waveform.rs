use anyhow::{ensure, Result};
use rodio::Source;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path, time::Duration};

const BINS: usize = 128;

#[derive(Debug, Serialize, Deserialize)]
pub struct Waveform {
    low: Vec<u8>,
    mid: Vec<u8>,
    high: Vec<u8>,
}

pub fn read(cache: &Path, path: &Path, tagged_duration: f64) -> Result<Waveform> {
    let metadata = path.metadata()?;
    let stamp = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let mut hash = Sha256::new();
    hash.update(path.as_os_str().as_encoded_bytes());
    hash.update(metadata.len().to_le_bytes());
    hash.update(stamp.to_le_bytes());
    hash.update(tagged_duration.to_bits().to_le_bytes());
    let file = cache.join(format!("{:x}.json", hash.finalize()));
    if let Ok(bytes) = std::fs::read(&file) {
        if let Ok(data) = serde_json::from_slice::<Waveform>(&bytes) {
            if [&data.low, &data.mid, &data.high]
                .iter()
                .all(|band| band.len() == BINS)
            {
                return Ok(data);
            }
        }
    }
    let mut source = musicos_desktop::local::decoded_audio::open(path)?;
    let duration = if tagged_duration.is_finite() && tagged_duration > 0. {
        tagged_duration
    } else {
        source
            .total_duration()
            .map_or(0., |value| value.as_secs_f64())
    };
    ensure!(
        duration.is_finite() && duration > 0.,
        "Audio duration unavailable"
    );
    let data = match sample(&mut source, duration)? {
        Some(data) => data,
        None => sequential(
            &mut musicos_desktop::local::decoded_audio::open(path)?,
            duration,
        )?,
    };
    if let Ok(bytes) = serde_json::to_vec(&data) {
        let _ = (|| -> Result<()> {
            std::fs::create_dir_all(cache)?;
            let temporary = file.with_extension(format!("{:016x}.tmp", rand::random::<u64>()));
            let result = (|| -> Result<()> {
                let mut output = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&temporary)?;
                output.write_all(&bytes)?;
                std::fs::rename(&temporary, file)?;
                Ok(())
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(temporary);
            }
            result
        })();
    }
    Ok(data)
}

fn sample(source: &mut impl Source<Item = f32>, length: f64) -> Result<Option<Waveform>> {
    let rate = source.sample_rate();
    let channels = usize::from(source.channels());
    ensure!(
        rate > 0 && (1..=32).contains(&channels),
        "Unsupported audio layout"
    );
    let mut bins = [[0.0_f32; 3]; BINS];
    for (index, bin) in bins.iter_mut().enumerate() {
        let bucket = length / BINS as f64;
        let sample_length = bucket.min(0.16);
        let offset = index as f64 * bucket + (bucket - sample_length) * 0.5;
        if source.try_seek(Duration::from_secs_f64(offset)).is_err() {
            return Ok(None);
        }
        let mut band = BandPeaks::new(rate);
        for _ in 0..(sample_length * f64::from(rate)).ceil() as usize {
            let Some(frame) = frame(source, channels) else {
                break;
            };
            band.add(frame);
        }
        *bin = band.peaks;
    }
    Ok(Some(normalized(bins)))
}

// Some decoders can stream a file but cannot seek within it.
fn sequential(source: &mut impl Source<Item = f32>, length: f64) -> Result<Waveform> {
    let rate = source.sample_rate();
    let channels = usize::from(source.channels());
    ensure!(
        length <= 1800.,
        "This long track requires a seekable decoder for waveform previews"
    );
    let mut bins = [[0.0; 3]; BINS];
    let bucket = length * f64::from(rate) / BINS as f64;
    let mut index = 0;
    let mut band = BandPeaks::new(rate);
    for (slot, bin) in bins.iter_mut().enumerate() {
        band.peaks = [0.; 3];
        let end = ((slot + 1) as f64 * bucket).round() as usize;
        while index < end {
            let Some(value) = frame(source, channels) else {
                break;
            };
            band.add(value);
            index += 1;
        }
        *bin = band.peaks;
    }
    Ok(normalized(bins))
}

fn normalized(bins: [[f32; 3]; BINS]) -> Waveform {
    let maximum = bins.iter().flatten().copied().fold(0.0_f32, f32::max);
    let normal = |band: usize| {
        bins.iter()
            .map(|bin| {
                if maximum <= 0.00001 {
                    0
                } else {
                    (255. * (bin[band] / maximum).clamp(0., 1.).sqrt()).round() as u8
                }
            })
            .collect()
    };
    Waveform {
        low: normal(0),
        mid: normal(1),
        high: normal(2),
    }
}

fn frame(source: &mut impl Iterator<Item = f32>, channels: usize) -> Option<f32> {
    let mut strongest = 0.0_f32;
    for _ in 0..channels {
        let value = source.next()?;
        if value.is_finite() && value.abs() > strongest.abs() {
            strongest = value;
        }
    }
    Some(strongest)
}

struct BandPeaks {
    low: f32,
    mid: f32,
    low_step: f32,
    mid_step: f32,
    peaks: [f32; 3],
}
impl BandPeaks {
    fn new(rate: u32) -> Self {
        let step = |frequency: f32| 1. - (-std::f32::consts::TAU * frequency / rate as f32).exp();
        Self {
            low: 0.,
            mid: 0.,
            low_step: step(180.),
            mid_step: step(2400.),
            peaks: [0.; 3],
        }
    }
    fn add(&mut self, frame: f32) {
        self.low += self.low_step * (frame - self.low);
        self.mid += self.mid_step * (frame - self.mid);
        for (peak, band) in
            self.peaks
                .iter_mut()
                .zip([self.low, self.mid - self.low, frame - self.mid])
        {
            *peak = peak.max(band.abs());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_frequency_bands_and_preserves_the_source() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("two-frequencies.wav");
        let cache = folder.path().join("cache");
        let sample_rate = 16000_u32;
        let length = sample_rate * 4 * 2;
        let mut audio = Vec::new();
        audio.extend(b"RIFF");
        audio.extend((36 + length).to_le_bytes());
        audio.extend(b"WAVEfmt ");
        audio.extend(16_u32.to_le_bytes());
        audio.extend(1_u16.to_le_bytes());
        audio.extend(1_u16.to_le_bytes());
        audio.extend(sample_rate.to_le_bytes());
        audio.extend((2 * sample_rate).to_le_bytes());
        audio.extend(2_u16.to_le_bytes());
        audio.extend(16_u16.to_le_bytes());
        audio.extend(b"data");
        audio.extend(length.to_le_bytes());
        for frame in 0..sample_rate * 4 {
            let frequency = if frame < sample_rate * 2 { 65. } else { 4000. };
            let phase = std::f32::consts::TAU * frequency * frame as f32 / sample_rate as f32;
            audio.extend(((phase.sin() * 22000.).round() as i16).to_le_bytes());
        }
        std::fs::write(&source, &audio).unwrap();

        let waveform = read(&cache, &source, 4.).unwrap();
        assert_eq!(waveform.low.len(), BINS);
        assert!(waveform.low[20] > waveform.high[20]);
        assert!(
            waveform.high[100] > waveform.low[100],
            "midpoint bands: {:?}, end bands: {:?}",
            [waveform.low[20], waveform.mid[20], waveform.high[20]],
            [waveform.low[100], waveform.mid[100], waveform.high[100]]
        );
        let reused = read(&cache, &source, 4.).unwrap();
        assert_eq!(reused.high, waveform.high);
        assert_eq!(std::fs::read(&source).unwrap(), audio);
        assert_eq!(std::fs::read_dir(cache).unwrap().count(), 1);
    }
}
