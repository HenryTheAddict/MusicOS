use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::models::TranscodeOptions;

pub fn validate_transcode_environment(options: &TranscodeOptions) -> Result<()> {
    if options.codec.trim().is_empty() || options.codec == "copy" {
        return Ok(());
    }
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .context("starting FFmpeg; install FFmpeg or select the Original profile")?;
    anyhow::ensure!(
        output.status.success(),
        "FFmpeg could not list its audio encoders"
    );
    let encoders = String::from_utf8_lossy(&output.stdout);
    let required = match options.codec.as_str() {
        "vorbis" => "libvorbis",
        "aac" => " aac ",
        "mp3" => "libmp3lame",
        "flac" => " flac ",
        value => anyhow::bail!("unsupported transcode codec: {value}"),
    };
    anyhow::ensure!(
        encoders.contains(required),
        "this FFmpeg build does not include the required {required} encoder"
    );
    Ok(())
}

pub struct PreparedFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub temporary_dir: Option<tempfile::TempDir>,
}

pub fn prepare_sync_file(
    source: &Path,
    relative_path: &Path,
    options: &TranscodeOptions,
) -> Result<PreparedFile> {
    let codec = if options.codec.trim().is_empty() {
        "copy"
    } else {
        options.codec.as_str()
    };
    if codec == "copy" {
        return Ok(PreparedFile {
            path: source.to_path_buf(),
            relative_path: relative_path.to_path_buf(),
            temporary_dir: None,
        });
    }
    let (extension, codec_args): (&str, Vec<String>) = match codec {
        "aac" => (
            "m4a",
            strings(&["-c:a", "aac", "-b:a", bitrate(options), "-ac", "2"]),
        ),
        "mp3" => (
            "mp3",
            strings(&["-c:a", "libmp3lame", "-b:a", bitrate(options), "-ac", "2"]),
        ),
        "flac" => (
            "flac",
            strings(&["-c:a", "flac", "-compression_level", "8"]),
        ),
        "vorbis" => {
            let args = if let Some(quality) = options.vorbis_quality {
                vec![
                    "-c:a".to_string(),
                    "libvorbis".to_string(),
                    "-q:a".to_string(),
                    format!("{:.1}", quality.clamp(0.0, 10.0)),
                ]
            } else {
                strings(&["-c:a", "libvorbis", "-b:a", bitrate(options)])
            };
            ("ogg", args)
        }
        value => anyhow::bail!("unsupported transcode codec: {value}"),
    };

    let temporary_dir = tempfile::Builder::new().prefix("musicos-sync-").tempdir()?;
    let output_name = format!(
        "{}.{extension}",
        source
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("track")
    );
    let output = temporary_dir.path().join(&output_name);
    let mut args = vec![
        "-v".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        source.display().to_string(),
        "-map".to_string(),
        "0:a:0".to_string(),
        "-map_metadata".to_string(),
        "0".to_string(),
    ];
    args.extend(codec_args);
    if let Some(channels) = options
        .channels
        .filter(|channels| matches!(channels, 1 | 2))
    {
        args.extend(["-ac".to_string(), channels.to_string()]);
    }
    if !options.sample_rate.is_empty() && options.sample_rate != "auto" {
        args.extend(["-ar".to_string(), options.sample_rate.clone()]);
    }
    args.push(output.display().to_string());
    let status = Command::new("ffmpeg")
        .args(&args)
        .status()
        .context("starting ffmpeg; install FFmpeg or choose Copy")?;
    anyhow::ensure!(status.success(), "ffmpeg exited with {status}");

    Ok(PreparedFile {
        path: output,
        relative_path: relative_path.with_file_name(output_name),
        temporary_dir: Some(temporary_dir),
    })
}

fn bitrate(options: &TranscodeOptions) -> &str {
    if options.bitrate.trim().is_empty() {
        "96k"
    } else {
        options.bitrate.as_str()
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}
