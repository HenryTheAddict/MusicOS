//! Experimental stem separation. A short-time Fourier transform estimates the
//! centre-panned, phase-coherent part of a stereo mix (usually the lead vocal)
//! and either keeps it (vocals) or removes it (instrumental) in real time.
use rustfft::{num_complex::Complex32, Fft, FftPlanner};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stems {
    #[default]
    Mix,
    Vocals,
    Instrumental,
}

const SIZE: usize = 2048;
const HOP: usize = SIZE / 4;
const BINS: usize = SIZE / 2 + 1;
const FADE: usize = 1024;
// The overlap-add pipeline emits each input frame exactly this many frames later.
const LATENCY: usize = SIZE;
// Entering separation from the undelayed path dips briefly instead of replaying audio.
const DIP: usize = 256;
const SAMPLE_RATE: f32 = 48_000.;
const VOCAL_LOW_HZ: f32 = 120.;
const VOCAL_HIGH_HZ: f32 = 12_000.;

pub struct StemSeparator {
    target: Stems,
    mode: Stems,
    mix: f32,
    delayed: bool,
    entering: Option<usize>,
    dry: [Vec<f32>; 2],
    dry_at: usize,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    input: [Vec<f32>; 2],
    output: [Vec<f32>; 2],
    ready: [Vec<f32>; 2],
    at: usize,
    frame: Vec<Complex32>,
    spectra: [Vec<Complex32>; 2],
    scratch: Vec<Complex32>,
    mask: Vec<f32>,
}

impl Default for StemSeparator {
    fn default() -> Self {
        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(SIZE);
        let inverse = planner.plan_fft_inverse(SIZE);
        let scratch = vec![
            Complex32::default();
            forward
                .get_inplace_scratch_len()
                .max(inverse.get_inplace_scratch_len())
        ];
        // Periodic sqrt-Hann for analysis and synthesis sums to a constant at 75% overlap.
        let window = (0..SIZE)
            .map(|i| (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / SIZE as f32).cos()).sqrt())
            .collect();
        Self {
            target: Stems::Mix,
            mode: Stems::Mix,
            mix: 0.,
            delayed: false,
            entering: None,
            dry: [vec![0.; LATENCY], vec![0.; LATENCY]],
            dry_at: 0,
            forward,
            inverse,
            window,
            input: [vec![0.; SIZE], vec![0.; SIZE]],
            output: [vec![0.; SIZE], vec![0.; SIZE]],
            ready: [vec![0.; HOP], vec![0.; HOP]],
            at: 0,
            frame: vec![Complex32::default(); SIZE],
            spectra: [
                vec![Complex32::default(); BINS],
                vec![Complex32::default(); BINS],
            ],
            scratch,
            mask: vec![0.; BINS],
        }
    }
}

impl StemSeparator {
    pub fn set(&mut self, stems: Stems) {
        if stems == self.target {
            return;
        }
        self.target = stems;
        if stems == Stems::Mix {
            return;
        }
        self.mode = stems;
        if !self.delayed {
            self.enter();
        }
    }
    /// Once separation has run, output stays on the delayed timeline until the
    /// next seek, so every later switch is a crossfade between two copies of
    /// the same moment instead of a jump between two moments.
    pub fn active(&self) -> bool {
        self.delayed
    }
    /// Drops buffered audio after a seek or a new source.
    pub fn reset(&mut self) {
        self.clear();
        self.delayed = false;
        self.entering = None;
        self.mix = 0.;
        if self.target != Stems::Mix {
            self.enter();
        }
    }
    fn enter(&mut self) {
        self.clear();
        self.delayed = true;
        self.entering = Some(0);
        self.mix = 1.;
    }
    fn clear(&mut self) {
        for c in 0..2 {
            self.input[c].fill(0.);
            self.output[c].fill(0.);
            self.ready[c].fill(0.);
            self.dry[c].fill(0.);
        }
        self.mask.fill(0.);
        self.at = 0;
        self.dry_at = 0;
    }
    pub fn process(&mut self, x: [f32; 2]) -> [f32; 2] {
        if !self.delayed {
            return x;
        }
        let separated = [self.ready[0][self.at], self.ready[1][self.at]];
        let slot = self.dry_at;
        let dry = [self.dry[0][slot], self.dry[1][slot]];
        self.dry[0][slot] = x[0];
        self.dry[1][slot] = x[1];
        self.dry_at = (slot + 1) % LATENCY;
        self.input[0][SIZE - HOP + self.at] = x[0];
        self.input[1][SIZE - HOP + self.at] = x[1];
        self.at += 1;
        if self.at == HOP {
            self.at = 0;
            self.separate();
        }
        let step = 1. / FADE as f32;
        self.mix = if self.target == Stems::Mix {
            (self.mix - step).max(0.)
        } else {
            (self.mix + step).min(1.)
        };
        let t = self.mix;
        let out = [
            dry[0] * (1. - t) + separated[0] * t,
            dry[1] * (1. - t) + separated[1] * t,
        ];
        let Some(n) = self.entering else {
            return out;
        };
        self.entering = (n + 1 < DIP + LATENCY + DIP).then_some(n + 1);
        if n < DIP {
            let g = 1. - n as f32 / DIP as f32;
            [x[0] * g, x[1] * g]
        } else if n < DIP + LATENCY {
            [0.; 2]
        } else {
            let g = (n - DIP - LATENCY) as f32 / DIP as f32;
            [out[0] * g, out[1] * g]
        }
    }
    fn separate(&mut self) {
        // Pack both channels into one complex transform: left in re, right in im.
        for i in 0..SIZE {
            self.frame[i] = Complex32::new(
                self.input[0][i] * self.window[i],
                self.input[1][i] * self.window[i],
            );
        }
        self.forward
            .process_with_scratch(&mut self.frame, &mut self.scratch);
        let (mut mid, mut side) = (0_f32, 0_f32);
        for k in 0..BINS {
            let a = self.frame[k];
            let b = self.frame[(SIZE - k) % SIZE].conj();
            let left = (a + b) * 0.5;
            let right = (a - b) * Complex32::new(0., -0.5);
            mid += (left + right).norm_sqr();
            side += (left - right).norm_sqr();
            self.spectra[0][k] = left;
            self.spectra[1][k] = right;
        }
        // A mono source is entirely centre; leave it untouched instead of muting it.
        let stereo = side > mid * 1e-5;
        let bin_hz = SAMPLE_RATE / SIZE as f32;
        let vocals = self.mode == Stems::Vocals;
        for k in 0..BINS {
            let (left, right) = (self.spectra[0][k], self.spectra[1][k]);
            let energy = left.norm_sqr() + right.norm_sqr();
            let coherence = if energy > 1e-12 {
                (2. * (left * right.conj()).re / energy).clamp(0., 1.)
            } else {
                0.
            };
            let band = stereo && (VOCAL_LOW_HZ..=VOCAL_HIGH_HZ).contains(&(k as f32 * bin_hz));
            let raw = if band { coherence.powi(8) } else { 0. };
            let previous = self.mask[k];
            let mask = if raw > previous {
                raw * 0.7 + previous * 0.3
            } else {
                raw * 0.4 + previous * 0.6
            };
            self.mask[k] = mask;
            let center = (left + right) * (0.5 * mask);
            let (left, right) = if !stereo {
                (left, right)
            } else if vocals {
                (center, center)
            } else {
                (left - center, right - center)
            };
            // Re-pack as left + i*right, keeping each channel's spectrum Hermitian.
            let i = Complex32::new(0., 1.);
            self.frame[k] = left + right * i;
            if k != 0 && k != SIZE / 2 {
                self.frame[SIZE - k] = left.conj() + right.conj() * i;
            }
        }
        self.inverse
            .process_with_scratch(&mut self.frame, &mut self.scratch);
        let scale = 1. / (SIZE as f32 * 2.);
        for i in 0..SIZE {
            let w = self.window[i] * scale;
            self.output[0][i] += self.frame[i].re * w;
            self.output[1][i] += self.frame[i].im * w;
        }
        for c in 0..2 {
            self.ready[c].copy_from_slice(&self.output[c][..HOP]);
            self.output[c].copy_within(HOP.., 0);
            self.output[c][SIZE - HOP..].fill(0.);
            self.input[c].copy_within(HOP.., 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(i: usize, hz: f32) -> f32 {
        (i as f32 * hz * std::f32::consts::TAU / SAMPLE_RATE).sin()
    }
    fn render(stems: Stems, signal: impl Fn(usize) -> [f32; 2]) -> Vec<[f32; 2]> {
        let mut separator = StemSeparator::default();
        separator.set(stems);
        (0..48_000).map(|i| separator.process(signal(i))).collect()
    }
    fn power(frames: &[[f32; 2]], channel: usize) -> f32 {
        let tail = &frames[24_000..];
        tail.iter().map(|f| f[channel] * f[channel]).sum::<f32>() / tail.len() as f32
    }
    // A centred 440 Hz "vocal" over a hard-left 660 Hz "guitar".
    fn song(i: usize) -> [f32; 2] {
        let vocal = tone(i, 440.) * 0.4;
        [vocal + tone(i, 660.) * 0.4, vocal]
    }

    #[test]
    fn mix_is_bit_transparent() {
        let frames = render(Stems::Mix, song);
        assert!(frames.iter().enumerate().all(|(i, f)| *f == song(i)));
    }
    #[test]
    fn instrumental_removes_the_centre_and_keeps_the_sides() {
        let frames = render(Stems::Instrumental, song);
        // Right only carried the centred vocal; left keeps the guitar's energy (0.4² / 2).
        assert!(
            power(&frames, 1) < 0.002,
            "vocal leaked: {}",
            power(&frames, 1)
        );
        assert!(
            (power(&frames, 0) - 0.08).abs() < 0.02,
            "guitar lost: {}",
            power(&frames, 0)
        );
    }
    #[test]
    fn vocals_keep_the_centre_and_drop_the_sides() {
        let frames = render(Stems::Vocals, song);
        let expected = 0.08;
        assert!(
            (power(&frames, 1) - expected).abs() < 0.02,
            "vocal lost: {}",
            power(&frames, 1)
        );
        assert!(
            (power(&frames, 0) - expected).abs() < 0.02,
            "guitar leaked: {}",
            power(&frames, 0)
        );
    }
    #[test]
    fn mono_sources_pass_through() {
        let frames = render(Stems::Instrumental, |i| [tone(i, 440.) * 0.4; 2]);
        assert!((power(&frames, 0) - 0.08).abs() < 0.01);
    }
    #[test]
    fn switching_modes_never_repeats_or_skips_audio() {
        // A mono ramp passes the separator unchanged, so each output frame must
        // be exactly the input from LATENCY frames earlier, across every switch.
        let ramp = |i: usize| [i as f32 / 100_000.; 2];
        let mut separator = StemSeparator::default();
        for i in 0..40_000 {
            match i {
                1_000 => separator.set(Stems::Instrumental),
                12_000 => separator.set(Stems::Mix),
                20_000 => separator.set(Stems::Vocals),
                30_000 => separator.set(Stems::Mix),
                _ => {}
            }
            let out = separator.process(ramp(i))[0];
            if i < 1_000 {
                assert_eq!(out, ramp(i)[0]);
            } else if i >= 1_000 + DIP + LATENCY + DIP {
                let expected = ramp(i - LATENCY)[0];
                assert!(
                    (out - expected).abs() < 1e-4,
                    "frame {i}: {out} vs {expected}"
                );
            } else if (1_000 + DIP..1_000 + DIP + LATENCY).contains(&i) {
                assert_eq!(out, 0., "frame {i} replayed buffered audio");
            }
        }
        assert!(separator.active());
    }
    #[test]
    fn seeking_drops_buffered_audio() {
        let mut separator = StemSeparator::default();
        separator.set(Stems::Instrumental);
        for i in 0..8_000 {
            separator.process(song(i));
        }
        separator.reset();
        for _ in 0..8_000 {
            assert!(separator.process([0.; 2]).iter().all(|v| v.abs() < 1e-6));
        }
    }
    #[test]
    fn seeking_in_full_mix_returns_to_bypass() {
        let mut separator = StemSeparator::default();
        separator.set(Stems::Vocals);
        for i in 0..8_000 {
            separator.process(song(i));
        }
        separator.set(Stems::Mix);
        separator.reset();
        assert!(!separator.active());
        assert_eq!(separator.process(song(3)), song(3));
    }
    #[test]
    fn old_preferences_default_to_mix() {
        assert_eq!(
            serde_json::from_str::<Stems>(r#""instrumental""#).unwrap(),
            Stems::Instrumental
        );
        assert_eq!(Stems::default(), Stems::Mix);
    }
}
