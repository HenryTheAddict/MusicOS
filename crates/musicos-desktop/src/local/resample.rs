use rodio::Source;
use std::iter::Fuse;

/// Linear conversion of a fixed-format decoded stream, with one phase across
/// packet boundaries. Construct after seeking and pull only on a worker.
pub struct ResampledFrames<I: Iterator, const CHANNELS: usize> {
    input: Fuse<I>,
    input_channels: usize,
    input_rate: u64,
    output_rate: u64,
    inverse_output_rate: f32,
    skip_samples: Option<usize>,
    phase: u64,
    left: Option<[f32; CHANNELS]>,
    right: Option<[f32; CHANNELS]>,
}

impl<I: Source<Item = f32>, const CHANNELS: usize> ResampledFrames<I, CHANNELS> {
    pub fn new(input: I, output_rate: u32) -> Self {
        let input_rate = u64::from(input.sample_rate());
        let input_channels = usize::from(input.channels());
        let output_rate = u64::from(output_rate);
        assert!(input_rate > 0 && output_rate > 0 && input_channels > 0 && CHANNELS > 0);
        // Integer decimation always lands on an input frame; it needs no lookahead.
        let skip_samples = if input_rate.is_multiple_of(output_rate) {
            usize::try_from((input_rate / output_rate - 1) * input_channels as u64).ok()
        } else {
            None
        };
        let mut frames = Self {
            input: input.fuse(),
            input_channels,
            input_rate,
            output_rate,
            inverse_output_rate: 1.0 / output_rate as f32,
            skip_samples,
            phase: 0,
            left: None,
            right: None,
        };
        if frames.skip_samples.is_none() {
            frames.left = frames.read_frame();
            if frames.left.is_some() {
                frames.right = frames.read_frame();
            }
        }
        frames
    }

    fn read_frame(&mut self) -> Option<[f32; CHANNELS]> {
        let mut frame = [0.; CHANNELS];
        let copied = self.input_channels.min(CHANNELS);
        for sample in &mut frame[..copied] {
            *sample = self.input.next()?;
        }
        for _ in copied..self.input_channels {
            self.input.next()?;
        }
        // Match rodio's channel mapping: mono is duplicated, extra input channels are dropped.
        if CHANNELS > 1 && self.input_channels == 1 {
            frame[1] = frame[0];
        }
        Some(frame)
    }
}

impl<I: Source<Item = f32>, const CHANNELS: usize> Iterator for ResampledFrames<I, CHANNELS> {
    type Item = [f32; CHANNELS];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(skip_samples) = self.skip_samples {
            let frame = self.read_frame()?;
            if skip_samples != 0 {
                let _ = self.input.nth(skip_samples - 1);
            }
            return Some(frame);
        }

        let mut left = self.left?;
        // Integer phase in units of 1/output_rate of an input frame; neither
        // decoder packet sizes nor floating-point rounding can shift the clock.
        while self.phase >= self.output_rate {
            left = self.right?;
            self.left = Some(left);
            self.right = self.read_frame();
            self.phase -= self.output_rate;
        }

        let mut output = left;
        if let Some(right) = self.right {
            let fraction = self.phase as f32 * self.inverse_output_rate;
            for channel in 0..CHANNELS {
                output[channel] += (right[channel] - left[channel]) * fraction;
            }
        }
        // Hold the last frame to the end of its interval. The total rounds up
        // once at EOF, never by more than one output frame.
        self.phase += self.input_rate;
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;

    #[test]
    fn preserves_channel_mapping_and_interpolates_across_frames() {
        let stereo = || SamplesBuffer::new(2, 24_000, vec![0., 0.5, 1., -0.5]);
        assert_eq!(
            ResampledFrames::<_, 2>::new(stereo(), 48_000).collect::<Vec<_>>(),
            [[0., 0.5], [0.5, 0.], [1., -0.5], [1., -0.5]]
        );
        assert_eq!(
            ResampledFrames::<_, 1>::new(stereo(), 48_000).collect::<Vec<_>>(),
            [[0.], [0.5], [1.], [1.]]
        );
        let mono = SamplesBuffer::new(1, 48_000, vec![0.25, -0.5]);
        assert_eq!(
            ResampledFrames::<_, 2>::new(mono, 48_000).collect::<Vec<_>>(),
            [[0.25, 0.25], [-0.5, -0.5]]
        );
        let surround =
            SamplesBuffer::new(3, 96_000, vec![0.25, -0.25, 1., 0., 0., 1., 0.5, -0.5, 1.]);
        assert_eq!(
            ResampledFrames::<_, 2>::new(surround, 48_000).collect::<Vec<_>>(),
            [[0.25, -0.25], [0.5, -0.5]]
        );
    }

    #[test]
    fn rounds_the_stream_length_once_at_eof() {
        for input_rate in [8_000, 44_100, 48_000, 96_000, 192_000] {
            for output_rate in [44_100, 48_000, 96_000] {
                for len in [0, 1, 2, 1_153] {
                    let mono = SamplesBuffer::new(1, input_rate, vec![0.25; len]);
                    let expected =
                        (len as u64 * u64::from(output_rate)).div_ceil(u64::from(input_rate));
                    assert_eq!(
                        ResampledFrames::<_, 2>::new(mono, output_rate).count(),
                        expected as usize
                    );
                }
            }
        }
    }
}
