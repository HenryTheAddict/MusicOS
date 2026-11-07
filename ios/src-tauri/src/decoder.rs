use std::{
    ffi::{c_char, CStr},
    path::Path,
};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint,
};
fn decode(source: &Path, destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if destination.exists() {
        return Ok(());
    }
    let mut hint = Hint::new();
    if let Some(ext) = source.extension().and_then(|v| v.to_str()) {
        hint.with_extension(ext);
    }
    let stream = MediaSourceStream::new(Box::new(std::fs::File::open(source)?), Default::default());
    let mut reader = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?
        .format;
    let track = reader.default_track().ok_or("No audio track")?;
    let id = track.id;
    let params = track.codec_params.clone();
    let rate = params.sample_rate.ok_or("Unknown sample rate")?;
    let channels = params.channels.ok_or("Unknown channels")?.count() as u16;
    let mut decoder = symphonia::default::get_codecs().make(&params, &DecoderOptions::default())?;
    let partial = destination.with_extension("part.wav");
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let spec = hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut output = hound::WavWriter::create(&partial, spec)?;
        loop {
            let packet = match reader.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break
                }
                Err(e) => return Err(e.into()),
            };
            if packet.track_id() != id {
                continue;
            }
            let audio = decoder.decode(&packet)?;
            let mut samples = SampleBuffer::<f32>::new(audio.capacity() as u64, *audio.spec());
            samples.copy_interleaved_ref(audio);
            for value in samples.samples() {
                output.write_sample(value.clamp(-1., 1.))?;
            }
        }
        output.finalize()?;
        std::fs::rename(&partial, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(partial);
    }
    result
}
#[no_mangle]
pub unsafe extern "C" fn musicos_prepare_audio(
    source: *const c_char,
    destination: *const c_char,
) -> i32 {
    if source.is_null() || destination.is_null() {
        return -1;
    }
    std::panic::catch_unwind(|| {
        let Ok(source) = CStr::from_ptr(source).to_str() else {
            return -1;
        };
        let Ok(destination) = CStr::from_ptr(destination).to_str() else {
            return -1;
        };
        if decode(Path::new(source), Path::new(destination)).is_ok() {
            0
        } else {
            -1
        }
    })
    .unwrap_or(-1)
}
