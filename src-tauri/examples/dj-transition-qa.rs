//! Isolated, muted native planner → decoder → sample-clocked mixer probe.
//! Reads the user's recordings; writes only an ephemeral test database.
use anyhow::{ensure, Context, Result};
use musicos_desktop::{
    local::{deck_playback::Transition, LocalPlaybackCommand, LocalPlaybackService},
    models::PlaybackPreferences,
};
use musicos_init::database::Db;
use musicos_tauri::{dj_plan, structure::Structure};
use std::time::{Duration, Instant};
fn wait(mut condition: impl FnMut() -> bool, seconds: u64) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(seconds);
    while !condition() {
        ensure!(
            Instant::now() < until,
            "Native playback condition timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
fn main() -> Result<()> {
    let source = std::env::args()
        .nth(1)
        .context("Pass source library database")?;
    let conn =
        rusqlite::Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let structures: Vec<(String, Structure)> =
        serde_json::from_slice(&std::fs::read("/tmp/musicos-structures-probe.json")?)?;
    let config = dj_plan::Config {
        enabled: true,
        ..Default::default()
    };
    let mut selected = None;
    'outer: for (name, a) in &structures {
        for (other, b) in &structures {
            if name == other {
                continue;
            }
            let plan = dj_plan::plan(
                dj_plan::Track {key_index:None,
                    duration: a.duration,
                    bpm: a.tempo,
                    structure: Some(a),
                },
                dj_plan::Track {key_index:None,
                    duration: b.duration,
                    bpm: b.tempo,
                    structure: Some(b),
                },
                &config,
                false,
            )
            .unwrap();
            if plan.confidence >= 0.5 {
                let album = |title: &str| {
                    conn.query_row(
                        "SELECT album FROM tracks WHERE title=? AND deleted_at IS NULL LIMIT 1",
                        [title],
                        |r| r.get::<_, String>(0),
                    )
                };
                if album(name)? == album(other)? {
                    selected = Some((name, other, plan));
                    break 'outer;
                }
            }
        }
    }
    let (outgoing, incoming, plan) =
        selected.context("No supported same-album pair; do not replace with a fabricated plan")?;
    let temporary = tempfile::tempdir()?;
    let db_path = temporary.path().join("playback.sqlite");
    let db = Db::open(db_path.to_str().unwrap())?;
    let mut ids = Vec::new();
    for title in [outgoing, incoming] {
        let(path,artist,album,duration):(String,String,String,i32)=conn.query_row("SELECT path,artist,album,duration FROM tracks WHERE title=? AND deleted_at IS NULL LIMIT 1",[title],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        ids.push(db.insert_track(
            &path,
            title,
            &artist,
            &album,
            duration,
            std::fs::metadata(&path)?.len() as i64,
            1,
        )?);
    }
    drop(db);
    let service = LocalPlaybackService::start_with_preferences(
        db_path,
        PlaybackPreferences {
            volume: 0.,
            ..Default::default()
        },
    )?;
    service.send(LocalPlaybackCommand::PlayTracks {
        ids: ids.clone(),
        start_index: 0,
    })?;
    wait(|| service.snapshot().queue.len() == 2, 10)?;
    ensure!(
        service.snapshot().backend == "MusicOS two-deck",
        "Native output unavailable"
    );
    service.send(LocalPlaybackCommand::SetDjMode(true))?;
    let prepare = || -> Result<()> {
        service.send(LocalPlaybackCommand::SeekAbsolute(
            plan.outgoing_cue - 3.,
        ))?;
        std::thread::sleep(Duration::from_millis(100));
        let s = service.snapshot();
        service.send(LocalPlaybackCommand::PlanTransition(Transition {
            outgoing: s.queue_entry_ids[0],
            incoming: s.queue_entry_ids[1],
            outgoing_cue: plan.outgoing_cue,
            incoming_cue: plan.incoming_cue,
            duration: plan.duration,
            tempo: plan.incoming_tempo,
            effect: serde_json::to_value(&plan.effect)?.as_str().unwrap().into(),
        }))?;
        wait(
            || {
                service
                    .snapshot()
                    .dj_transition
                    .is_some_and(|t| t.status == "scheduled")
            },
            10,
        )
    };
    prepare()?;
    service.send(LocalPlaybackCommand::Pause)?;
    std::thread::sleep(Duration::from_millis(100));
    let paused = service.snapshot().position;
    std::thread::sleep(Duration::from_millis(150));
    ensure!(
        (service.snapshot().position - paused).abs() < 0.01,
        "Pause moved source clock"
    );
    service.send(LocalPlaybackCommand::PlayPause)?;
    let mut active = false;
    wait(
        || {
            let s = service.snapshot();
            active |= s.dj_transition.as_ref().is_some_and(|t| t.active);
            s.queue_index == Some(1)
        },
        25,
    )?;
    ensure!(active, "Deck promoted without observed active overlap");
    let promoted = service.snapshot().position;
    let expected=plan.incoming_cue+plan.duration*plan.incoming_tempo;
    ensure!((promoted-expected).abs()<0.3,"Incoming source accounting differs: {promoted} versus {expected}");
    service.send(LocalPlaybackCommand::PlayTracks {
        ids,
        start_index: 0,
    })?;
    wait(|| service.snapshot().queue_index == Some(0), 10)?;
    prepare()?;
    wait(
        || service.snapshot().dj_transition.is_some_and(|t| t.active),
        15,
    )?;
    service.send(LocalPlaybackCommand::SeekAbsolute(0.))?;
    wait(|| {let s=service.snapshot();s.queue_index==Some(0)&&s.position<0.5&&!s.dj_transition.is_some_and(|t|t.active&&t.seconds>0.)}, 5)?;
    service.send(LocalPlaybackCommand::Pause)?;
    println!(
        "{}",
        serde_json::json!({"outgoing":outgoing,"incoming":incoming,"plan":plan,"scheduled":true,"automatic_activation":active,"deck_promotion":true,"promoted_source_position":promoted,"pause_freezes_source_clock":true,"seek_cancels_active_overlap":true,"volume":0,"user_queue_modified":false,"audible_listening":false})
    );
    Ok(())
}
