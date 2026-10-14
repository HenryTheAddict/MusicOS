//! Read-only comparison against cached local recordings. Does not mutate analysis or playback.
fn main() -> anyhow::Result<()> {
    let db = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Pass library database path"))?;
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut query=conn.prepare("SELECT t.title,t.path,d.result FROM tracks t JOIN desktop_dsp_v1 d ON d.track_id=t.id WHERE lower(t.album) IN ('discovery','junk','currents') AND t.deleted_at IS NULL")?;
    let rows = query.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        let (title, path, json) = row?;
        let old: musicos_tauri::analysis::Features = serde_json::from_str(&json)?;
        let start = std::time::Instant::now();
        let new = musicos_tauri::structure::scan(
            std::path::Path::new(&path),
            old.bpm,
            old.bpm_confidence,
            || true,
        )?;
        println!(
            "{}",
            serde_json::json!({"title":title,"bpm":old.bpm,"full_track_bpm":new.tempo,"tempo_confidence":old.bpm_confidence,"old_beat_confidence":old.structure.map(|s|s.beat_confidence),"new_beat_confidence":new.beat_confidence,"beats":new.beats.len(),"phrases":new.phrases.len(),"strong_phrases":new.phrases.iter().filter(|p|p.confidence>=0.5).count(),"repeated_regions":new.regions.iter().filter(|r|r.repeated).count(),"milliseconds":start.elapsed().as_millis()})
        );
        results.push((title, new));
    }
    std::fs::write(
        "/tmp/musicos-structures-probe.json",
        serde_json::to_vec(&results)?,
    )?;
    let config = musicos_tauri::dj_plan::Config {
        enabled: true,
        ..Default::default()
    };
    let mut plans = Vec::new();
    for (name, a) in &results {
        for (other, b) in &results {
            if name == other {
                continue;
            }
            let p = musicos_tauri::dj_plan::plan(
                musicos_tauri::dj_plan::Track {key_index:None,
                    duration: a.duration,
                    bpm: a.tempo,
                    structure: Some(a),
                },
                musicos_tauri::dj_plan::Track {key_index:None,
                    duration: b.duration,
                    bpm: b.tempo,
                    structure: Some(b),
                },
                &config,
                false,
            )
            .unwrap();
            plans.push(serde_json::json!({"from":name,"to":other,"reason":p.reason,"cue":p.outgoing_cue,"overlap":p.duration,"confidence":p.confidence}));
        }
    }
    std::fs::write(
        "/tmp/musicos-dj-plan-report.json",
        serde_json::to_vec_pretty(&plans)?,
    )?;
    println!(
        "{}",
        serde_json::json!({"total_pairs":plans.len(),"musical_plans":plans.iter().filter(|p|p["confidence"].as_f64().unwrap_or(0.)>0.).count(),"examples":plans.iter().filter(|p|p["confidence"].as_f64().unwrap_or(0.)>0.).take(8).collect::<Vec<_>>()})
    );
    Ok(())
}
