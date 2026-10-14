use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use musicos_desktop::api_client::ApiClient;
use musicos_desktop::models::TranscodeOptions;
use musicos_desktop::sync::{resolve_sync_files, sync_files};
use serde_json::{json, Value};

const HELP: &str = "MusicOS Agent CLI

Usage:
  musicos-agent device status [options]
  musicos-agent devices list [options]
  musicos-agent sync plan [options]
  musicos-agent sync start [options]

Options:
  --target <url>         MusicOS HTTP endpoint (default: MUSICOS_SYNC_TARGET)
  --source <directory>   Local music root (default: MUSICOS_SYNC_DIR or ./music)
  --file <path>          Sync one file; repeat for multiple files
  --codec <name>         copy|vorbis|aac|mp3|flac (default: copy)
  --bitrate <rate>       96k, 128k, 192k, etc.
  --sample-rate <rate>   auto|44100|48000|96000 (default: auto)
  --timeout <ms>         HTTP timeout in milliseconds (default: 120000)
  --json                 Print the final result as JSON
  --json-lines           Stream progress events as JSON Lines
  -h, --help             Show help

Exit codes: 0 success, 2 usage error, 3 device unreachable, 4 sync failure.";

#[derive(Debug)]
struct Options {
    noun: String,
    verb: String,
    target: String,
    source: PathBuf,
    files: Vec<PathBuf>,
    transcode: TranscodeOptions,
    timeout_ms: u64,
    json: bool,
    json_lines: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<u8> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || matches!(args[0].as_str(), "help" | "-h" | "--help") {
        println!("{HELP}");
        return Ok(0);
    }
    let options = parse(args)?;
    let client = match ApiClient::with_timeout(
        options.target.clone(),
        Duration::from_millis(options.timeout_ms),
    ) {
        Ok(client) => client,
        Err(error) => bail!("{error}"),
    };

    if (options.noun == "device" && options.verb == "status")
        || (options.noun == "devices" && options.verb == "list")
    {
        return match client.status() {
            Ok(status) => {
                let device = json!({
                    "target": options.target,
                    "reachable": true,
                    "status": status
                });
                print_value(
                    if options.noun == "devices" {
                        json!({ "devices": [device] })
                    } else {
                        device
                    },
                    options.json || options.json_lines,
                );
                Ok(0)
            }
            Err(error) => {
                let device = json!({
                    "target": options.target,
                    "reachable": false,
                    "error": error.to_string()
                });
                print_value(
                    if options.noun == "devices" {
                        json!({ "devices": [device] })
                    } else {
                        device
                    },
                    true,
                );
                Ok(3)
            }
        };
    }

    if options.noun != "sync" || !matches!(options.verb.as_str(), "plan" | "start") {
        bail!("unknown command");
    }
    let requested = (!options.files.is_empty()).then_some(options.files.as_slice());
    let files = resolve_sync_files(&options.source, requested)?;
    if options.verb == "plan" {
        let total_bytes = files
            .iter()
            .map(std::fs::metadata)
            .collect::<std::io::Result<Vec<_>>>()?
            .iter()
            .map(std::fs::Metadata::len)
            .sum::<u64>();
        print_value(
            json!({
                "ok": true,
                "syncDir": std::fs::canonicalize(&options.source)?,
                "target": options.target,
                "scoped": requested.is_some(),
                "files": files.len(),
                "totalBytes": total_bytes,
                "unreadable": 0,
                "transcode": {
                    "codec": options.transcode.codec,
                    "bitrate": options.transcode.bitrate,
                    "sampleRate": options.transcode.sample_rate
                }
            }),
            options.json || options.json_lines,
        );
        return Ok(0);
    }

    let cancel = AtomicBool::new(false);
    let progress = sync_files(
        &options.source,
        requested,
        &client,
        &options.transcode,
        &cancel,
        |state| {
            if options.json_lines {
                println!(
                    "{}",
                    serde_json::to_string(&json!({"event": "progress", "state": state}))
                        .expect("serializing progress")
                );
            } else if !options.json {
                eprintln!(
                    "[progress] {}/{} {}",
                    state.uploaded + state.skipped + state.failed,
                    state.found,
                    state.current
                );
            }
        },
    )
    .context("synchronizing music")?;
    if !options.json_lines {
        print_value(serde_json::to_value(&progress)?, options.json);
    }
    Ok(if progress.failed > 0 || progress.cancel_requested {
        4
    } else {
        0
    })
}

fn parse(args: Vec<String>) -> Result<Options> {
    if args.len() < 2 {
        bail!("expected a command and action");
    }
    let noun = args[0].clone();
    let verb = args[1].clone();
    let mut target = std::env::var("MUSICOS_SYNC_TARGET")
        .unwrap_or_else(|_| "http://127.0.0.1:18080".to_string());
    let mut source = std::env::var_os("MUSICOS_SYNC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("music"));
    let mut files = Vec::new();
    let mut codec = "copy".to_string();
    let mut bitrate = String::new();
    let mut sample_rate = "auto".to_string();
    let mut timeout_ms = 120_000;
    let mut json = false;
    let mut json_lines = false;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--target" => target = value(&args, &mut index, "--target")?,
            "--source" => source = value(&args, &mut index, "--source")?.into(),
            "--file" => files.push(value(&args, &mut index, "--file")?.into()),
            "--codec" => codec = value(&args, &mut index, "--codec")?,
            "--bitrate" => bitrate = value(&args, &mut index, "--bitrate")?,
            "--sample-rate" => sample_rate = value(&args, &mut index, "--sample-rate")?,
            "--timeout" => {
                timeout_ms = value(&args, &mut index, "--timeout")?
                    .parse()
                    .context("--timeout must be a positive integer")?;
                if timeout_ms == 0 {
                    bail!("--timeout must be a positive integer");
                }
            }
            "--json" => json = true,
            "--json-lines" => json_lines = true,
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            option => bail!("unknown option: {option}"),
        }
        index += 1;
    }
    if !matches!(codec.as_str(), "copy" | "vorbis" | "aac" | "mp3" | "flac") {
        bail!("--codec must be copy, vorbis, aac, mp3, or flac");
    }
    if !matches!(sample_rate.as_str(), "auto" | "44100" | "48000" | "96000") {
        bail!("--sample-rate must be auto, 44100, 48000, or 96000");
    }
    Ok(Options {
        noun,
        verb,
        target,
        source,
        files,
        transcode: TranscodeOptions {
            codec,
            bitrate,
            sample_rate,
            vorbis_quality: None,
            channels: None,
        },
        timeout_ms,
        json,
        json_lines,
    })
}

fn value(args: &[String], index: &mut usize, option: &str) -> Result<String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .with_context(|| format!("{option} requires a value"))
}

fn print_value(value: Value, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("serializing result")
        );
    } else if let Some(files) = value.get("files").and_then(Value::as_u64) {
        println!(
            "files={files} bytes={} unreadable={}",
            value.get("totalBytes").and_then(Value::as_u64).unwrap_or(0),
            value.get("unreadable").and_then(Value::as_u64).unwrap_or(0)
        );
    } else if let Some(found) = value.get("found").and_then(Value::as_u64) {
        println!(
            "found={found} uploaded={} skipped={} failed={}",
            value.get("uploaded").and_then(Value::as_u64).unwrap_or(0),
            value.get("skipped").and_then(Value::as_u64).unwrap_or(0),
            value.get("failed").and_then(Value::as_u64).unwrap_or(0)
        );
    } else {
        println!(
            "{} {}",
            value
                .get("target")
                .and_then(Value::as_str)
                .unwrap_or("device"),
            if value
                .get("reachable")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "reachable"
            } else {
                "unreachable"
            }
        );
    }
}
