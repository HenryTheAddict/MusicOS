use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use musicos_init::database::{Db, MetadataEnrichmentStatus, MetadataUpdate, Track};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::models::{ArtworkPreferences, EnrichmentPreferences};

const MUSICBRAINZ_SEARCH: &str = "https://musicbrainz.org/ws/2/recording/";
const CACHE_MAX_AGE_SECONDS: i64 = 30 * 24 * 60 * 60;
const MAX_ARTWORK_BYTES: u64 = 20 * 1024 * 1024;
const USER_AGENT: &str = "MusicOS/0.1 (https://github.com/HenryTheAddict/MusicOS)";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichmentProgress {
    pub processed: usize,
    pub total: usize,
    pub current: String,
    pub applied: usize,
    pub review: usize,
    pub unmatched: usize,
    pub artwork: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichmentRunReport {
    pub processed: usize,
    pub applied: usize,
    pub review: usize,
    pub unmatched: usize,
    pub protected: usize,
    pub artwork: usize,
    pub failed: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataCandidate {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i32>,
    #[serde(default)] pub artwork_path: Option<String>,
    pub recording_mbid: String,
    pub release_mbid: Option<String>,
    pub release_group_mbid: Option<String>,
    pub confidence: f32,
    #[serde(default)] pub album_artist:Option<String>,
    #[serde(default)] pub track_number:Option<i32>,
    #[serde(default)] pub disc_number:Option<i32>,
}

/// Stable match identity excludes confidence, timestamps and volatile cache paths.
pub fn candidate_fingerprint(candidate:&MetadataCandidate)->String{
    let clean=|text:&str|text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let artwork=candidate.artwork_path.as_deref().map(|path|std::fs::metadata(path).ok().filter(|m|m.len()<=20*1024*1024).and_then(|_|std::fs::read(path).ok()).map(|bytes|format!("{:x}",Sha256::digest(bytes))).unwrap_or_else(||Path::new(path).file_name().unwrap_or_default().to_string_lossy().into_owned()));
    let base=format!("{:x}",Sha256::digest(serde_json::to_vec(&(clean(&candidate.title),clean(&candidate.artist),clean(&candidate.album),candidate.year,&candidate.recording_mbid,&candidate.release_mbid,&candidate.release_group_mbid,artwork)).unwrap_or_default()));if candidate.track_number.is_some()||candidate.disc_number.is_some()||candidate.album_artist.is_some(){format!("{:x}",Sha256::digest(serde_json::to_vec(&(base,&candidate.album_artist,candidate.track_number,candidate.disc_number)).unwrap_or_default()))}else{base}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataReviewItem {
    pub track_id: i64,
    pub current_title: String,
    pub current_artist: String,
    pub current_album: String,
    pub current_year: Option<i32>,
    pub current_artwork: Option<String>,
    pub revision: i64,
    pub candidate_token: String,
    pub candidate: MetadataCandidate,
}

pub fn load_metadata_review_items(db_path: &Path, limit: usize) -> Result<Vec<MetadataReviewItem>> {
    let db = Db::open(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    let mut items = Vec::new();
    for status in db.metadata_enrichment_statuses("review", limit)? {
        let Some(track) = db.get_track_by_id(status.track_id)? else {
            continue;
        };
        let Some(candidate_json) = status.candidate_json else {
            continue;
        };
        let candidate: MetadataCandidate = serde_json::from_str(&candidate_json)?;
        items.push(MetadataReviewItem {
            track_id: track.id,
            current_title: track.title,
            current_artist: track.artist,
            current_album: track.album,
            current_year: db.metadata_values(track.id)?["year"].as_i64().map(|n|n as i32),
            current_artwork: track.cover_art_path,
            revision:db.metadata_values(track.id)?["revision"].as_i64().unwrap_or(0),
            candidate_token:candidate_json,
            candidate,
        });
    }
    Ok(items)
}

pub fn accept_metadata_review(db_path: &Path, track_id: i64) -> Result<bool> {
    let db = Db::open(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    let status = db
        .metadata_enrichment_statuses("review", 500)?
        .into_iter()
        .find(|status| status.track_id == track_id)
        .context("metadata review item is no longer available")?;
    let candidate: MetadataCandidate = serde_json::from_str(
        status
            .candidate_json
            .as_deref()
            .context("metadata review item has no candidate")?,
    )?;
    // Accepting a review is an explicit user edit, protected from later automation.
    let changed = db.update_track_metadata_override(
        &[track_id],
        &MetadataUpdate {
            title: Some(candidate.title),
            artist: Some(candidate.artist),
            album: (!candidate.album.is_empty()).then_some(candidate.album),
            year: candidate.year,
            ..MetadataUpdate::default()
        },
    )? > 0;
    db.set_metadata_enrichment_state(track_id, "applied")?;
    Ok(changed)
}

pub fn dismiss_metadata_review(db_path: &Path, track_id: i64) -> Result<bool> {
    let db = Db::open(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    db.set_metadata_enrichment_state(track_id, "kept")
}

#[derive(Debug, Deserialize)]
struct RecordingSearchResponse {
    #[serde(default)]
    recordings: Vec<RecordingResult>,
}

#[derive(Debug, Deserialize)]
struct RecordingResult {
    id: String,
    title: String,
    length: Option<u64>,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<ArtistCredit>,
    #[serde(default)]
    releases: Vec<ReleaseResult>,
}

#[derive(Debug, Deserialize)]
struct ArtistCredit {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseResult {
    id: String,
    title: String,
    date: Option<String>,
    #[serde(rename = "release-group")]
    release_group: Option<ReleaseGroupResult>,
}

#[derive(Debug, Deserialize)]
struct ReleaseGroupResult {
    id: String,
}

pub fn enrich_library<F>(
    db_path: &Path,
    track_ids: &[i64],
    preferences: &EnrichmentPreferences,
    artwork_preferences: &ArtworkPreferences,
    cancel: &AtomicBool,
    mut progress: F,
) -> Result<EnrichmentRunReport>
where
    F: FnMut(EnrichmentProgress),
{
    find_metadata_suggestions(db_path,track_ids,preferences,artwork_preferences,cancel,false,progress)
}

pub fn find_metadata_suggestions<F>(
    db_path: &Path,
    track_ids: &[i64],
    preferences: &EnrichmentPreferences,
    artwork_preferences: &ArtworkPreferences,
    cancel: &AtomicBool,
    force: bool,
    mut progress: F,
) -> Result<EnrichmentRunReport>
where
    F: FnMut(EnrichmentProgress),
{
    let db = Db::open_background(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let mut limiter = MusicBrainzLimiter::default();
    let mut report = EnrichmentRunReport::default();


    let mut known=HashMap::new();
    if !force {for state in ["review","kept","ignored","applied","protected"] {for status in db.metadata_enrichment_statuses(state,100000)? {known.insert(status.track_id,status.source_modified_at);}}}
    let mut release_cache=HashMap::<String,Option<serde_json::Value>>::new();
    for track_id in track_ids {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        let Some(track) = db.get_track_by_id(*track_id)? else {
            continue;
        };
        if !force && known.get(track_id)==Some(&track.modified_at) { report.processed+=1;emit_progress(&mut progress,&report,track_ids.len(),&track.title);continue; }
        if db.metadata_override_origin(*track_id)?.as_deref() == Some("user") {
            report.protected += 1;
            report.processed += 1;
            record_status(&db, &track, "protected", 0.0, None, None)?;
            emit_progress(&mut progress, &report, track_ids.len(), &track.title);
            continue;
        }

        let tagged=lofty::read_from_path(&track.path).ok();let release_id=tagged.as_ref().and_then(|file|lofty::file::TaggedFileExt::primary_tag(file)).and_then(|tag|tag.get_string(&lofty::tag::ItemKey::MusicBrainzReleaseId));
        let album_key=release_cache_key(&track,release_id);
        let result=(||->Result<Option<MetadataCandidate>>{if let Some(release)=release_cache.get(&album_key).and_then(Option::as_ref){if let Some(candidate)=candidate_from_album(&track,release){return Ok(Some(candidate));} if release_id.is_some(){return Ok(None);}}
            if let Some(id)=release_id {if !release_cache.contains_key(&album_key){let release=release_details(&db,&client,&mut limiter,id,cancel)?;release_cache.insert(album_key.clone(),Some(release));}return Ok(release_cache.get(&album_key).and_then(Option::as_ref).and_then(|release|candidate_from_album(&track,release)));}
            let seed=search_candidate(&db,&client,&mut limiter,&track,cancel)?;
            if !release_cache.contains_key(&album_key)&&!track.album.trim().is_empty()&&!track.album.eq_ignore_ascii_case("unknown album") {let release=seed.as_ref().filter(|c|c.confidence>=preferences.review_threshold&&text_similarity(&c.album,&track.album)>0.85).and_then(|c|c.release_mbid.as_deref()).map(|id|release_details(&db,&client,&mut limiter,id,cancel)).transpose().ok().flatten();release_cache.insert(album_key.clone(),release);}
            Ok(release_cache.get(&album_key).and_then(Option::as_ref).and_then(|release|candidate_from_album(&track,release)).or(seed))})();
        match result {
            Ok(Some(mut candidate)) if candidate.confidence >= preferences.review_threshold => {
                if preferences.download_artwork {
                    if let Some(release)=candidate.release_mbid.as_deref() {
                        candidate.artwork_path=download_release_art(&db,&client,release,artwork_preferences.preferred_download_pixels,artwork_preferences.disk_cache_megabytes).ok().flatten().map(|p|p.to_string_lossy().into_owned());
                    }
                }
                if !db.metadata_candidate_ignored(track.id,&candidate_fingerprint(&candidate))? {report.review += 1;}
                record_status(
                    &db,
                    &track,
                    "review",
                    candidate.confidence,
                    Some(&candidate),
                    None,
                )?;
            }
            Ok(Some(candidate)) => {
                report.unmatched += 1;
                record_status(
                    &db,
                    &track,
                    "unmatched",
                    candidate.confidence,
                    Some(&candidate),
                    None,
                )?;
            }
            Ok(None) => {
                report.unmatched += 1;
                record_status(&db, &track, "unmatched", 0.0, None, None)?;
            }
            Err(error) => {
                report.failed += 1;
                record_status(&db, &track, "failed", 0.0, None, Some(format!("{error:#}")))?;
            }
        }
        report.processed += 1;
        emit_progress(&mut progress, &report, track_ids.len(), &track.title);
    }
    Ok(report)
}

fn emit_progress(
    progress: &mut impl FnMut(EnrichmentProgress),
    report: &EnrichmentRunReport,
    total: usize,
    current: &str,
) {
    progress(EnrichmentProgress {
        processed: report.processed,
        total,
        current: current.to_string(),
        applied: report.applied,
        review: report.review,
        unmatched: report.unmatched,
        artwork: report.artwork,
        failed: report.failed,
    });
}

fn search_candidate(
    db: &Db,
    client: &Client,
    limiter: &mut MusicBrainzLimiter,
    track: &Track,
    cancel: &AtomicBool,
) -> Result<Option<MetadataCandidate>> {
    let strict = musicbrainz_query(track);
    let mut best = search_recording_query(db,client,limiter,track,&strict,cancel)?;
    let relaxed = recording_query(track,false);
    // One bounded fallback. No query expansion may choose a recording on a
    // release name alone; title, credits and duration still drive review.
    if relaxed != strict && best.as_ref().is_none_or(|candidate|candidate.confidence<0.9) && !cancel.load(Ordering::Relaxed) {
        let fallback=search_recording_query(db,client,limiter,track,&relaxed,cancel)?;
        if fallback.as_ref().is_some_and(|candidate| text_similarity(&track.title,&candidate.title)>=0.8 && text_similarity(primary_artist(&track.artist),&candidate.artist)>=0.45)
            && fallback.as_ref().is_some_and(|candidate|best.as_ref().is_none_or(|previous|candidate.confidence>previous.confidence)) {best=fallback;}
    }
    Ok(best)
}

fn search_recording_query(db:&Db,client:&Client,limiter:&mut MusicBrainzLimiter,track:&Track,query:&str,cancel:&AtomicBool)->Result<Option<MetadataCandidate>> {
    let cache_key = format!("musicbrainz-recording-v2:{}", sha256(query));
    let body = if let Some(body) = db.cached_provider_response(&cache_key, CACHE_MAX_AGE_SECONDS)? {
        body
    } else {
        limiter.wait(cancel)?;
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let mut last_error = None;
        let mut body = None;
        for attempt in 0..3 {
            let response = client
                .get(MUSICBRAINZ_SEARCH)
                .query(&[("query", query), ("fmt", "json"), ("limit", "5")])
                .send();
            match response {
                Ok(response) if response.status().is_success() => {
                    body = Some(response.text()?);
                    break;
                }
                Ok(response) => {
                    last_error = Some(anyhow::anyhow!(
                        "MusicBrainz returned HTTP {}",
                        response.status()
                    ));
                }
                Err(error) => last_error = Some(error.into()),
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(500 * (attempt + 1) as u64));
            }
        }
        let body = body.ok_or_else(|| {
            last_error.unwrap_or_else(|| anyhow::anyhow!("MusicBrainz lookup failed"))
        })?;
        db.cache_provider_response(&cache_key, "musicbrainz", &body)?;
        body
    };
    let response: RecordingSearchResponse =
        serde_json::from_str(&body).context("decoding MusicBrainz recording search response")?;
    Ok(response
        .recordings
        .into_iter()
        .filter(|recording| recording_duration_matches(track,recording))
        .filter_map(|recording| candidate_from_recording(track, recording))
        .max_by(|left, right| left.confidence.total_cmp(&right.confidence)))
}

fn release_details(db:&Db,client:&Client,limiter:&mut MusicBrainzLimiter,id:&str,cancel:&AtomicBool)->Result<serde_json::Value>{
    anyhow::ensure!(id.len()==36&&id.bytes().all(|b|b.is_ascii_hexdigit()||b==b'-'),"Invalid release identity");let key=format!("musicbrainz-album-v1:{id}");let body=if let Some(body)=db.cached_provider_response(&key,CACHE_MAX_AGE_SECONDS)?{body}else{limiter.wait(cancel)?;anyhow::ensure!(!cancel.load(Ordering::Relaxed),"Lookup canceled");let body=client.get(format!("https://musicbrainz.org/ws/2/release/{id}")).query(&[("inc","recordings+artist-credits+release-groups"),("fmt","json")]).send()?.error_for_status()?.text()?;anyhow::ensure!(body.len()<8*1024*1024,"Release response is too large");db.cache_provider_response(&key,"musicbrainz",&body)?;body};let value:serde_json::Value=serde_json::from_str(&body)?;anyhow::ensure!(value["id"]==id,"Release identity changed");Ok(value)
}
fn candidate_from_album(track:&Track,release:&serde_json::Value)->Option<MetadataCandidate>{
    let mut candidates=Vec::new();for medium in release["media"].as_array()?{for entry in medium["tracks"].as_array()?{let recording=&entry["recording"];let title=entry["title"].as_str().or(recording["title"].as_str())?;let credits=entry.get("artist-credit").or(recording.get("artist-credit")).or(release.get("artist-credit"))?;let artist=credits.as_array()?.iter().filter_map(|a|a["name"].as_str()).collect::<Vec<_>>().join(" & ");let duration=entry["length"].as_u64().or(recording["length"].as_u64());let title_score=text_similarity(&track.title,title);let duration_score=duration.map(|ms|(1.-(track.duration as f32-ms as f32/1000.).abs()/8.).clamp(0.,1.)).unwrap_or(0.5);let position=entry["position"].as_i64().unwrap_or(0) as i32;let number_score=if track.track_number>0&&track.track_number==position {1.}else{0.};
    let score=title_score*0.55+text_similarity(&track.artist,&artist)*0.2+duration_score*0.2+number_score*0.05;
    // Number alone never identifies a recording, particularly incomplete editions.
    if title_score<0.5||duration.is_some_and(|ms|(track.duration as f32-ms as f32/1000.).abs()>30.){continue;}
    candidates.push(MetadataCandidate{title:title.into(),artist,album:release["title"].as_str()?.into(),year:parse_year(release["date"].as_str()),artwork_path:None,recording_mbid:recording["id"].as_str()?.into(),release_mbid:release["id"].as_str().map(str::to_owned),release_group_mbid:release["release-group"]["id"].as_str().map(str::to_owned),confidence:score,album_artist:release["artist-credit"].as_array().map(|credits|credits.iter().filter_map(|c|c["name"].as_str()).collect::<Vec<_>>().join(" & ")),track_number:(position>0).then_some(position),disc_number:medium["position"].as_i64().map(|n|n as i32)});}}
    candidates.sort_by(|a,b|b.confidence.total_cmp(&a.confidence));if candidates.len()>1&&(candidates[0].confidence-candidates[1].confidence)<0.03{return None;}candidates.into_iter().next()
}

fn same_tagged_album(tagged: &str, candidate: &str) -> bool {
    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let tagged = normalize(tagged);
    tagged.is_empty() || tagged == "unknown album" || tagged == normalize(candidate)
}

fn candidate_from_recording(
    track: &Track,
    recording: RecordingResult,
) -> Option<MetadataCandidate> {
    let artist = recording
        .artist_credit
        .iter()
        .map(|credit| credit.name.as_str())
        .collect::<Vec<_>>()
        .join(" & ");
    let release = recording.releases.iter().max_by(|left, right| {
        text_similarity(release_title(&track.album), release_title(&left.title))
            .total_cmp(&text_similarity(release_title(&track.album), release_title(&right.title)))
    });
    let album = release
        .map(|release| release.title.clone())
        .unwrap_or_default();
    let title_score = text_similarity(&track.title, &recording.title);
    let artist_score = text_similarity(&track.artist, &artist);
    let album_score = if unknown_album(&track.album) || album.is_empty() {
        0.5
    } else {
        text_similarity(release_title(&track.album), release_title(&album))
    };
    let duration_score = recording
        .length
        .map(|length| {
            let delta = (track.duration as f32 - length as f32 / 1000.0).abs();
            (1.0 - delta / 15.0).clamp(0.0, 1.0)
        })
        .unwrap_or(0.65);
    let confidence =
        title_score * 0.45 + artist_score * 0.28 + album_score * 0.12 + duration_score * 0.15;
    Some(MetadataCandidate {
        artwork_path: None,
        title: recording.title,
        artist,
        album,
        year: release.and_then(|release| parse_year(release.date.as_deref())),
        recording_mbid: recording.id,
        release_mbid: release.map(|release| release.id.clone()),
        release_group_mbid: release
            .and_then(|release| release.release_group.as_ref())
            .map(|group| group.id.clone()),
        confidence: confidence.clamp(0.0, 1.0),album_artist:None,track_number:None,disc_number:None,
    })
}

fn recording_duration_matches(track:&Track,recording:&RecordingResult)->bool {track.duration<=0||recording.length.is_none_or(|ms|(track.duration as f32-ms as f32/1000.).abs()<=15_f32.max(track.duration as f32*0.05))}
fn unknown_album(album:&str)->bool {album.trim().is_empty()||album.trim().eq_ignore_ascii_case("unknown album")}
fn primary_artist(artist:&str)->&str {artist.split([';','+']).next().unwrap_or(artist).trim()}
fn release_title(album:&str)->&str {
    let title=album.trim();let lower=title.to_lowercase();
    for suffix in [" - single"," - ep"] {if lower.ends_with(suffix){return title[..title.len()-suffix.len()].trim();}}
    title
}
fn release_cache_key(track:&Track,release_id:Option<&str>)->String {
    if let Some(id)=release_id{return format!("release:{id}");}
    if unknown_album(&track.album){return format!("loose:{}",track.id);}
    format!("album:{}:{}",primary_artist(&track.artist).to_lowercase(),track.album.to_lowercase())
}
fn musicbrainz_query(track: &Track) -> String {recording_query(track,true)}
fn recording_query(track:&Track,strict:bool)->String {
    let mut parts=vec![format!("recording:\"{}\"",escape_query(&track.title))];
    if !track.artist.trim().is_empty()&&!track.artist.eq_ignore_ascii_case("unknown artist") {
        parts.push(format!("artist:\"{}\"",escape_query(if strict {&track.artist}else{primary_artist(&track.artist)})));
    }
    if strict&&!unknown_album(&track.album) {parts.push(format!("release:\"{}\"",escape_query(release_title(&track.album))));}
    parts.join(" AND ")
}

fn escape_query(value: &str) -> String {
    value.replace('\\', " ").replace('"', " ")
}

fn text_similarity(left: &str, right: &str) -> f32 {
    let left = normalized_tokens(left);
    let right = normalized_tokens(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }
    let intersection = left.iter().filter(|token| right.contains(token)).count();
    let union = left.len() + right.len() - intersection;
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

fn normalized_tokens(value: &str) -> Vec<String> {
    let mut tokens = value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    tokens.sort();
    tokens.dedup();
    tokens
}

fn parse_year(value: Option<&str>) -> Option<i32> {
    value?
        .get(..4)?
        .parse::<i32>()
        .ok()
        .filter(|year| (1000..=9999).contains(year))
}

fn artwork_needs_upgrade(track: &Track, threshold: u32) -> bool {
    let Some(path) = track.cover_art_path.as_deref().map(Path::new) else {
        return true;
    };
    if !path.is_file() {
        return true;
    }
    image::image_dimensions(path)
        .map(|(width, height)| width.min(height) < threshold)
        .unwrap_or(true)
}

fn download_release_art(
    db: &Db,
    client: &Client,
    release_id: &str,
    preferred_pixels: u32,
    cache_megabytes: u32,
) -> Result<Option<PathBuf>> {
    let size = if preferred_pixels >= 1000 { 1200 } else { 500 };
    let directory = db.cover_art_dir().join("musicbrainz");
    std::fs::create_dir_all(&directory)?;
    let destination = directory.join(format!("{release_id}.png"));
    if destination.is_file() {
        return Ok(Some(destination));
    }
    let url = format!("https://coverartarchive.org/release/{release_id}/front-{size}");
    let response = client.get(url).send()?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = response.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_ARTWORK_BYTES)
    {
        anyhow::bail!("cover artwork exceeds the 20 MB safety limit");
    }
    let bytes = response.bytes()?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_ARTWORK_BYTES,
        "cover artwork exceeds the 20 MB safety limit"
    );
    let image = image::load_from_memory(&bytes).context("decoding downloaded cover artwork")?;
    anyhow::ensure!(
        image.width() >= 100 && image.height() >= 100,
        "cover artwork is too small"
    );
    let image = if image.width().max(image.height()) > preferred_pixels.max(500) {
        image.resize(
            preferred_pixels.max(500),
            preferred_pixels.max(500),
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        image
    };
    let temporary = directory.join(format!(".{release_id}.download.png"));
    image.save_with_format(&temporary, image::ImageFormat::Png)?;
    std::fs::rename(&temporary, &destination)?;
    enforce_remote_artwork_cache(db, &directory, cache_megabytes)?;
    Ok(Some(destination))
}

fn enforce_remote_artwork_cache(db: &Db, directory: &Path, cache_megabytes: u32) -> Result<()> {
    let limit = u64::from(cache_megabytes.max(64)) * 1024 * 1024;
    let mut files = std::fs::read_dir(directory)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            metadata.is_file().then_some((
                entry.path(),
                metadata.len(),
                metadata.modified().unwrap_or(UNIX_EPOCH),
            ))
        })
        .collect::<Vec<_>>();
    let mut total = files.iter().map(|(_, size, _)| *size).sum::<u64>();
    if total <= limit {
        return Ok(());
    }
    files.sort_by_key(|(_, _, modified)| *modified);
    for (path, size, _) in files {
        if total <= limit {
            break;
        }
        db.clear_cover_art_path(&path)?;
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
    Ok(())
}

fn record_status(
    db: &Db,
    track: &Track,
    state: &str,
    confidence: f32,
    candidate: Option<&MetadataCandidate>,
    error: Option<String>,
) -> Result<()> {
    let state=if state=="review"&&candidate.is_some_and(|c|db.metadata_candidate_ignored(track.id,&candidate_fingerprint(c)).unwrap_or(false)){"ignored"}else{state};
    db.record_metadata_enrichment_status(&MetadataEnrichmentStatus {
        track_id: track.id,
        state: state.to_string(),
        provider: "musicbrainz".to_string(),
        confidence,
        candidate_json: candidate.map(serde_json::to_string).transpose()?,
        recording_mbid: candidate.map(|candidate| candidate.recording_mbid.clone()),
        release_mbid: candidate.and_then(|candidate| candidate.release_mbid.clone()),
        release_group_mbid: candidate.and_then(|candidate| candidate.release_group_mbid.clone()),
        error,
        checked_at: unix_seconds(),
        source_modified_at: track.modified_at,
    })
}

#[derive(Debug, Default)]
struct MusicBrainzLimiter {
    last_request: Option<Instant>,
}

impl MusicBrainzLimiter {
    fn wait(&mut self, cancel: &AtomicBool) -> Result<()> {
        if let Some(last_request) = self.last_request {
            let remaining = Duration::from_secs(1).saturating_sub(last_request.elapsed());
            let started = Instant::now();
            while started.elapsed() < remaining {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        self.last_request = Some(Instant::now());
        Ok(())
    }
}

fn sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignored_match_identity_is_stable_but_changes_with_recording_or_fields(){
        let mut c=MetadataCandidate{title:" Song ".into(),artist:"Artist".into(),album:"Album".into(),year:Some(2020),artwork_path:None,recording_mbid:"recording-1".into(),release_mbid:Some("release-1".into()),release_group_mbid:None,confidence:0.8,album_artist:None,track_number:None,disc_number:None};
        let first=candidate_fingerprint(&c);c.confidence=0.3;c.title="song".into();assert_eq!(first,candidate_fingerprint(&c));c.release_mbid=Some("release-2".into());assert_ne!(first,candidate_fingerprint(&c));c.release_mbid=Some("release-1".into());c.year=Some(2021);assert_ne!(first,candidate_fingerprint(&c));
    }


    fn track(title: &str, artist: &str, album: &str, duration: i32) -> Track {
        Track {
            id: 1,
            path: "/music/test.ogg".to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            track_number: 1,
            duration,
            file_size: 1,
            modified_at: 1,
            indexed_at: 1,
            cover_art_path: None,
            cover_art_updated_at: None,
            cover_art_source_modified_at: None,
            favorited: false,
            play_count: 0,
            last_played: None,
            rating: 0,
        }
    }

    #[test]fn album_matching_uses_disc_positions_and_rejects_ambiguous_recordings(){let release=serde_json::json!({"id":"release","title":"Album","date":"2020-01-01","artist-credit":[{"name":"Artist"}],"media":[{"position":2,"tracks":[{"position":1,"title":"Song","length":200000,"recording":{"id":"recording","title":"Song"}}]}]});let candidate=candidate_from_album(&track("Song","Artist","Album",200),&release).unwrap();assert_eq!(candidate.disc_number,Some(2));assert_eq!(candidate.track_number,Some(1));let mut duplicate=release.clone();let row=duplicate["media"][0]["tracks"][0].clone();duplicate["media"][0]["tracks"].as_array_mut().unwrap().push(row);assert!(candidate_from_album(&track("Song","Artist","Album",200),&duplicate).is_none());assert!(candidate_from_album(&track("Different","Artist","Album",200),&release).is_none());}
    #[test]fn singles_queries_and_cache_do_not_merge_loose_releases(){
        let mut loose=track("Song","Artist; Guest","Unknown Album",200);
        assert_eq!(recording_query(&loose,false),"recording:\"Song\" AND artist:\"Artist\"");
        assert!(!musicbrainz_query(&loose).contains("release:"));
        let first=release_cache_key(&loose,None);loose.id=2;assert_ne!(first,release_cache_key(&loose,None));
        assert_ne!(release_cache_key(&loose,Some("edition-a")),release_cache_key(&loose,Some("edition-b")));
        loose.album="Song - Single".into();assert!(musicbrainz_query(&loose).ends_with("release:\"Song\""));
        assert_eq!(release_title("Album (Deluxe)"),"Album (Deluxe)");
        let candidate=candidate_from_recording(&track("Song","Artist","Unknown Album",200),RecordingResult{id:"recording".into(),title:"Song".into(),length:Some(200000),artist_credit:vec![ArtistCredit{name:"Artist".into()}],releases:vec![]}).unwrap();
        assert!(candidate.confidence>=0.9,"Unknown release should not penalize an exact recording: {}",candidate.confidence);
        let recording=RecordingResult{id:"other-version".into(),title:"Song".into(),length:Some(300000),artist_credit:vec![],releases:vec![]};assert!(!recording_duration_matches(&loose,&recording));
    }

    #[test]
    fn exact_candidate_scores_above_auto_apply_threshold() {
        let candidate = candidate_from_recording(
            &track("Digital Love", "Daft Punk", "Discovery", 301),
            RecordingResult {
                id: "recording".to_string(),
                title: "Digital Love".to_string(),
                length: Some(301_000),
                artist_credit: vec![ArtistCredit {
                    name: "Daft Punk".to_string(),
                }],
                releases: vec![ReleaseResult {
                    id: "release".to_string(),
                    title: "Discovery".to_string(),
                    date: Some("2001-03-12".to_string()),
                    release_group: Some(ReleaseGroupResult {
                        id: "group".to_string(),
                    }),
                }],
            },
        )
        .unwrap();
        assert!(candidate.confidence >= 0.90);
        assert_eq!(candidate.year, Some(2001));
    }

    #[test]
    fn unrelated_candidate_stays_below_review_threshold() {
        let candidate = candidate_from_recording(
            &track("Digital Love", "Daft Punk", "Discovery", 301),
            RecordingResult {
                id: "recording".to_string(),
                title: "Blue Monday".to_string(),
                length: Some(450_000),
                artist_credit: vec![ArtistCredit {
                    name: "New Order".to_string(),
                }],
                releases: vec![ReleaseResult {
                    id: "release".to_string(),
                    title: "Power, Corruption & Lies".to_string(),
                    date: Some("1983".to_string()),
                    release_group: None,
                }],
            },
        )
        .unwrap();
        assert!(candidate.confidence < 0.72);
    }

    #[test]
    fn recording_match_cannot_replace_tagged_album_edition() {
        assert!(same_tagged_album(" CHROMAKOPIA ","chromakopia"));
        assert!(!same_tagged_album("CHROMAKOPIA","CHROMAKOPIA (Live at Camp Flog Gnaw)"));
        assert!(!same_tagged_album("Discovery (Deluxe)","Discovery"));
        assert!(same_tagged_album("", "Discovery"));
    }

    #[test]
    fn query_escapes_quotes_and_omits_unknown_values() {
        let value = musicbrainz_query(&track("Say \"Hello\"", "Unknown Artist", "", 10));
        assert_eq!(value, "recording:\"Say  Hello \"");
    }
}

#[cfg(test)] mod suggestion_policy_tests {
 use super::*;
 #[test] fn high_confidence_matches_wait_for_review_and_dismissals_require_explicit_retry(){
  let dir=tempfile::tempdir().unwrap();let path=dir.path().join("library.db");let db=Db::open(path.to_str().unwrap()).unwrap();let id=db.insert_track("/song.flac","digital love","Daft Punk","Discovery",301,1,1).unwrap();let track=db.get_track_by_id(id).unwrap().unwrap();
  let response=serde_json::json!({"recordings":[{"id":"recording","title":"Digital Love","length":301000,"artist-credit":[{"name":"Daft Punk"}],"releases":[{"id":"release","title":"Discovery","date":"2001-03-12","release-group":{"id":"group"}}]}]});
  db.cache_provider_response(&format!("musicbrainz:{}",sha256(&musicbrainz_query(&track))),"musicbrainz",&response.to_string()).unwrap();
  let prefs=EnrichmentPreferences{download_artwork:false,..Default::default()};let cancel=AtomicBool::new(false);
  let report=find_metadata_suggestions(&path,&[id],&prefs,&ArtworkPreferences::default(),&cancel,false,|_|{}).unwrap();assert_eq!(report.review,1);assert_eq!(report.applied,0);assert_eq!(db.get_track_by_id(id).unwrap().unwrap().title,"digital love");assert!(db.metadata_override_origin(id).unwrap().is_none());
  db.set_metadata_enrichment_state(id,"kept").unwrap();assert_eq!(find_metadata_suggestions(&path,&[id],&prefs,&ArtworkPreferences::default(),&cancel,false,|_|{}).unwrap().review,0);
  assert_eq!(find_metadata_suggestions(&path,&[id],&prefs,&ArtworkPreferences::default(),&cancel,true,|_|{}).unwrap().review,1);
  let candidate:MetadataCandidate=serde_json::from_str(db.metadata_enrichment_statuses("review",10).unwrap()[0].candidate_json.as_deref().unwrap()).unwrap();let identity=candidate_fingerprint(&candidate);let conn=musicos_init::database::Connection::open(&path).unwrap();conn.execute("INSERT INTO metadata_ignored_candidates VALUES(?,?,?,1)",rusqlite::params![id,identity,serde_json::to_string(&candidate).unwrap()]).unwrap();
  assert_eq!(find_metadata_suggestions(&path,&[id],&prefs,&ArtworkPreferences::default(),&cancel,true,|_|{}).unwrap().review,0);assert_eq!(Db::open(path.to_str().unwrap()).unwrap().metadata_enrichment_statuses("ignored",10).unwrap().len(),1);

 }
}
