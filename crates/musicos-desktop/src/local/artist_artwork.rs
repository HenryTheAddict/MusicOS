use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use musicos_init::database::Db;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const USER_AGENT: &str = "MusicOS/0.1 (https://github.com/HenryTheAddict/MusicOS)";
const CACHE_MAX_AGE: i64 = 90 * 24 * 60 * 60;
const NEGATIVE_CACHE_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);
const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct ArtistSearchResponse {
    #[serde(default)]
    artists: Vec<ArtistSearchResult>,
}

#[derive(Debug, Deserialize)]
struct ArtistSearchResult {
    id: String,
    name: String,
    #[serde(default)]
    score: u32,
}

#[derive(Debug, Deserialize)]
struct ArtistLookupResponse {
    #[serde(default)]
    relations: Vec<UrlRelation>,
}

#[derive(Debug, Deserialize)]
struct UrlRelation {
    #[serde(rename = "type")]
    relation_type: String,
    url: RelatedUrl,
}

#[derive(Debug, Deserialize)]
struct RelatedUrl {
    resource: String,
}

pub fn cached_artist_portrait(db_path: &Path, artist: &str) -> Option<PathBuf> {
    let path = portrait_path(db_path, artist);
    path.is_file().then_some(path)
}

pub fn ensure_artist_portraits<F>(
    db_path: &Path,
    artists: &[String],
    cancel: &AtomicBool,
    mut progress: F,
) -> Result<HashMap<String, PathBuf>>
where
    F: FnMut(usize, usize, &str, Option<&Path>),
{
    let db = Db::open(
        db_path
            .to_str()
            .context("library database path is not valid UTF-8")?,
    )?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let mut limiter = MusicBrainzLimiter::default();
    let mut portraits = HashMap::new();

    for (index, artist) in artists.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if let Some(path) = cached_artist_portrait(db_path, artist) {
            portraits.insert(artist.clone(), path);
            progress(
                index + 1,
                artists.len(),
                artist,
                portraits.get(artist).map(PathBuf::as_path),
            );
            continue;
        }
        if negative_cache_is_fresh(db_path, artist) {
            progress(index + 1, artists.len(), artist, None);
            continue;
        }
        match fetch_artist_portrait(&db, &client, &mut limiter, db_path, artist, cancel) {
            Ok(Some(path)) => {
                let _ = std::fs::remove_file(negative_path(db_path, artist));
                portraits.insert(artist.clone(), path);
                progress(
                    index + 1,
                    artists.len(),
                    artist,
                    portraits.get(artist).map(PathBuf::as_path),
                );
            }
            Ok(None) => {
                mark_missing(db_path, artist);
                progress(index + 1, artists.len(), artist, None);
            }
            Err(error) => {
                log::warn!("desktop-artist-artwork: {artist}: {error:#}");
                progress(index + 1, artists.len(), artist, None);
            }
        }
    }
    Ok(portraits)
}

fn fetch_artist_portrait(
    db: &Db,
    client: &Client,
    limiter: &mut MusicBrainzLimiter,
    db_path: &Path,
    artist: &str,
    cancel: &AtomicBool,
) -> Result<Option<PathBuf>> {
    let query = format!("artist:\"{}\"", artist.replace(['\\', '"'], " "));
    let search_body = cached_get(
        db,
        client,
        limiter,
        cancel,
        "musicbrainz-artist-search",
        "https://musicbrainz.org/ws/2/artist/",
        &[("query", query.as_str()), ("fmt", "json"), ("limit", "5")],
    )?;
    let response: ArtistSearchResponse =
        serde_json::from_str(&search_body).context("decoding MusicBrainz artist search")?;
    let expected = normalized_name(artist);
    let Some(candidate) = response
        .artists
        .into_iter()
        .filter(|candidate| normalized_name(&candidate.name) == expected || candidate.score >= 95)
        .max_by_key(|candidate| {
            (
                normalized_name(&candidate.name) == expected,
                candidate.score,
            )
        })
    else {
        return Ok(None);
    };

    let lookup_url = format!("https://musicbrainz.org/ws/2/artist/{}/", candidate.id);
    let lookup_body = cached_get(
        db,
        client,
        limiter,
        cancel,
        "musicbrainz-artist-lookup",
        &lookup_url,
        &[("inc", "url-rels"), ("fmt", "json")],
    )?;
    let lookup: ArtistLookupResponse =
        serde_json::from_str(&lookup_body).context("decoding MusicBrainz artist relations")?;
    let Some(entity_id) = lookup
        .relations
        .iter()
        .find(|relation| relation.relation_type == "wikidata")
        .and_then(|relation| relation.url.resource.rsplit('/').next())
        .filter(|id| {
            id.starts_with('Q') && id[1..].chars().all(|character| character.is_ascii_digit())
        })
    else {
        return Ok(None);
    };

    let wikidata_url =
        format!("https://www.wikidata.org/wiki/Special:EntityData/{entity_id}.json?flavor=simple");
    let wikidata_body = cached_plain_get(db, client, "wikidata-artist", &wikidata_url)?;
    let wikidata: serde_json::Value =
        serde_json::from_str(&wikidata_body).context("decoding Wikidata artist entity")?;
    let Some(filename) = wikidata
        .pointer(&format!(
            "/entities/{entity_id}/claims/P18/0/mainsnak/datavalue/value"
        ))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };

    let commons_body = client
        .get("https://commons.wikimedia.org/w/api.php")
        .query(&[
            ("action", "query"),
            ("format", "json"),
            ("prop", "imageinfo"),
            ("iiprop", "url"),
            ("iiurlwidth", "900"),
            ("titles", &format!("File:{filename}")),
        ])
        .send()?
        .error_for_status()?
        .text()?;
    let commons: serde_json::Value =
        serde_json::from_str(&commons_body).context("decoding Wikimedia image response")?;
    let Some(image_url) = commons
        .pointer("/query/pages")
        .and_then(serde_json::Value::as_object)
        .and_then(|pages| pages.values().next())
        .and_then(|page| page.pointer("/imageinfo/0"))
        .and_then(|info| info.get("thumburl").or_else(|| info.get("url")))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };

    let response = client.get(image_url).send()?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IMAGE_BYTES)
    {
        anyhow::bail!("artist portrait exceeds the 20 MB safety limit");
    }
    let bytes = response.bytes()?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_IMAGE_BYTES,
        "artist portrait exceeds the 20 MB safety limit"
    );
    let image = image::load_from_memory(&bytes).context("decoding artist portrait")?;
    anyhow::ensure!(
        image.width() >= 160 && image.height() >= 160,
        "artist portrait is too small"
    );
    let image = image.resize(900, 900, image::imageops::FilterType::Lanczos3);
    let destination = portrait_path(db_path, artist);
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension("download.png");
    image.save_with_format(&temporary, image::ImageFormat::Png)?;
    std::fs::rename(&temporary, &destination)?;
    Ok(Some(destination))
}

fn cached_get(
    db: &Db,
    client: &Client,
    limiter: &mut MusicBrainzLimiter,
    cancel: &AtomicBool,
    provider: &str,
    url: &str,
    query: &[(&str, &str)],
) -> Result<String> {
    let key = format!("{provider}:{}", cache_hash(&format!("{url}{query:?}")));
    if let Some(body) = db.cached_provider_response(&key, CACHE_MAX_AGE)? {
        return Ok(body);
    }
    let mut last_error = None;
    let mut body = None;
    for attempt in 0..3 {
        limiter.wait(cancel)?;
        match client.get(url).query(query).send() {
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
            std::thread::sleep(Duration::from_millis(700 * (attempt + 1) as u64));
        }
    }
    let body = body.ok_or_else(|| {
        last_error.unwrap_or_else(|| anyhow::anyhow!("MusicBrainz artist lookup failed"))
    })?;
    db.cache_provider_response(&key, provider, &body)?;
    Ok(body)
}

fn cached_plain_get(db: &Db, client: &Client, provider: &str, url: &str) -> Result<String> {
    let key = format!("{provider}:{}", cache_hash(url));
    if let Some(body) = db.cached_provider_response(&key, CACHE_MAX_AGE)? {
        return Ok(body);
    }
    let body = client.get(url).send()?.error_for_status()?.text()?;
    db.cache_provider_response(&key, provider, &body)?;
    Ok(body)
}

#[derive(Default)]
struct MusicBrainzLimiter {
    last_request: Option<Instant>,
}

impl MusicBrainzLimiter {
    fn wait(&mut self, cancel: &AtomicBool) -> Result<()> {
        if let Some(last) = self.last_request {
            let remaining = Duration::from_millis(1100).saturating_sub(last.elapsed());
            let start = Instant::now();
            while start.elapsed() < remaining {
                if cancel.load(Ordering::Relaxed) {
                    anyhow::bail!("artist artwork lookup cancelled");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        self.last_request = Some(Instant::now());
        Ok(())
    }
}

fn cache_directory(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("artist-art")
        .join("v1")
}

fn portrait_path(db_path: &Path, artist: &str) -> PathBuf {
    cache_directory(db_path).join(format!("{}.png", cache_hash(&normalized_name(artist))))
}

fn negative_path(db_path: &Path, artist: &str) -> PathBuf {
    cache_directory(db_path).join(format!("{}.missing", cache_hash(&normalized_name(artist))))
}

fn negative_cache_is_fresh(db_path: &Path, artist: &str) -> bool {
    std::fs::metadata(negative_path(db_path, artist))
        .and_then(|metadata| metadata.modified())
        .and_then(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .map_err(std::io::Error::other)
        })
        .is_ok_and(|age| age < NEGATIVE_CACHE_AGE)
}

fn mark_missing(db_path: &Path, artist: &str) {
    let path = negative_path(db_path, artist);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, b"No MusicBrainz/Wikidata portrait was found.");
}

fn normalized_name(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn cache_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_stable_across_artist_punctuation() {
        assert_eq!(normalized_name("Tyler,  The Creator"), "tyler the creator");
        assert_eq!(normalized_name("TYLER — THE CREATOR"), "tyler the creator");
    }

    #[test]
    fn wikidata_portrait_claim_path_is_the_expected_shape() {
        let value: serde_json::Value = serde_json::from_str(
            r#"{"entities":{"Q42":{"claims":{"P18":[{"mainsnak":{"datavalue":{"value":"Portrait.jpg"}}}]}}}}"#,
        )
        .unwrap();
        assert_eq!(
            value
                .pointer("/entities/Q42/claims/P18/0/mainsnak/datavalue/value")
                .and_then(serde_json::Value::as_str),
            Some("Portrait.jpg")
        );
    }
}
