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

/// Read recording identity from the local file without decoding its audio.
#[no_mangle]
pub unsafe extern "C" fn musicos_read_audio_info(
    source: *const c_char,
    output: *mut c_char,
    capacity: usize,
) -> i32 {
    if source.is_null() || output.is_null() || capacity < 2 {
        return -1;
    }
    std::panic::catch_unwind(|| {
        use symphonia::core::meta::StandardTagKey;
        let Ok(path) = CStr::from_ptr(source).to_str() else {
            return -1;
        };
        let Ok(file) = std::fs::File::open(path) else {
            return -1;
        };
        let mut hint = Hint::new();
        if let Some(extension) = Path::new(path).extension().and_then(|v| v.to_str()) {
            hint.with_extension(extension);
        }
        let Ok(mut probed) = symphonia::default::get_probe().format(
            &hint,
            MediaSourceStream::new(Box::new(file), Default::default()),
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) else {
            return -1;
        };
        let mut result = serde_json::Map::new();
        let mut read = |revision: &symphonia::core::meta::MetadataRevision| {
            for tag in revision.tags() {
                let key = match tag.std_key {
                    Some(StandardTagKey::IdentIsrc) => "isrc",
                    Some(StandardTagKey::Bpm) => "bpm",
                    _ if tag.key.eq_ignore_ascii_case("ISRC")
                        || tag.key.eq_ignore_ascii_case("TSRC") => "isrc",
                    _ => continue,
                };
                result.insert(key.into(), serde_json::Value::String(tag.value.to_string()));
            }
        };
        if let Some(metadata) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
            read(metadata);
        }
        if let Some(metadata) = probed.format.metadata().current() {
            read(metadata);
        }
        let Ok(bytes) = serde_json::to_vec(&result) else {
            return -1;
        };
        if bytes.len() + 1 > capacity {
            return -1;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output.cast::<u8>(), bytes.len());
        *output.add(bytes.len()) = 0;
        bytes.len() as i32
    })
    .unwrap_or(-1)
}

// Share the desktop's bounded, SDR perceptual color extraction. WebKit can
// forbid canvas readback for downloaded file URLs, so canvas is not authoritative.
#[path = "../../../src-tauri/src/artwork_colors.rs"]
mod artwork_colors;
#[no_mangle]
pub unsafe extern "C" fn musicos_artwork_colors(source:*const c_char,output:*mut c_char,capacity:usize)->i32{
    if source.is_null()||output.is_null()||capacity==0{return -1;}
    std::panic::catch_unwind(||{
        let Ok(path)=CStr::from_ptr(source).to_str()else{return -1;};
        let Ok(mut reader)=image::io::Reader::open(path).and_then(|r|r.with_guessed_format())else{return -1;};
        let mut limits=image::io::Limits::default();limits.max_alloc=Some(64*1024*1024);reader.limits(limits);
        let Ok(image)=reader.decode()else{return -1;};let colors=artwork_colors::swatches(&image.thumbnail(64,64).to_rgba8());
        let Ok(bytes)=serde_json::to_vec(&colors)else{return -1;};if bytes.len()+1>capacity{return -1;}
        std::ptr::copy_nonoverlapping(bytes.as_ptr(),output.cast::<u8>(),bytes.len());*output.add(bytes.len())=0;bytes.len()as i32
    }).unwrap_or(-1)
}

#[path = "../../../src-tauri/src/transition_energy.rs"]
mod transition_energy;

/// Profiles contain at most 64 quarter-second bins. The output has three slots:
/// outgoing cue, overlap, and incoming cue, in source seconds.
#[no_mangle]
pub unsafe extern "C" fn musicos_transition_cue(
    tail: *const f32,
    tail_count: usize,
    tail_start: f32,
    head: *const f32,
    head_count: usize,
    duration: f32,
    output: *mut f32,
) {
    if tail.is_null() || head.is_null() || output.is_null()
        || tail_count > 64 || head_count > 64 || !duration.is_finite() || duration <= 0.
    {
        return;
    }
    use transition_energy::Envelope;
    let envelope = |start, values| Envelope { start, step: 0.25, values };
    let tail_values = std::slice::from_raw_parts(tail, tail_count);
    let head_values = std::slice::from_raw_parts(head, head_count);
    let fade = transition_energy::tail(envelope(tail_start, tail_values), duration);
    let overlap = fade.map_or(2_f32.min(duration / 4.), |fade| fade.overlap);
    let cue = fade.map_or(duration - overlap, |fade| fade.cue);
    let intro = transition_energy::intro(envelope(0., head_values));
    std::ptr::copy_nonoverlapping([cue, overlap, intro].as_ptr(), output, 3);
}
