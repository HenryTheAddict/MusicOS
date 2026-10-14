use std::time::Duration;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::{Client, Response};
use serde::{de::DeserializeOwned, Serialize};

use crate::models::{LibrarySnapshot, LibrarySource, Mix, PlaybackSnapshot, Playlist, Track};

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    client: Client,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Self::with_timeout(base_url, Duration::from_secs(120))
    }

    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Result<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
            anyhow::bail!("MusicOS target must start with http:// or https://");
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(timeout)
            .build()?;
        Ok(Self { base_url, client })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        decode(self.client.get(self.url(path)).send(), path)
    }

    pub fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        decode(self.client.post(self.url(path)).json(body).send(), path)
    }

    pub fn put<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        decode(self.client.put(self.url(path)).json(body).send(), path)
    }

    pub fn patch<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        decode(self.client.patch(self.url(path)).json(body).send(), path)
    }

    pub fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        decode(self.client.delete(self.url(path)).send(), path)
    }

    pub fn status(&self) -> Result<serde_json::Value> {
        self.get("/api/status")
    }

    pub fn wait_until_ready(&self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        let mut last_error = None;
        while Instant::now() < deadline {
            match self.status() {
                Ok(_) => return Ok(()),
                Err(error) => last_error = Some(error),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(last_error.unwrap_or_else(|| anyhow!("MusicOS runtime did not become ready")))
            .context("waiting for the local MusicOS runtime")
    }

    pub fn control(&self, command: &str) -> Result<serde_json::Value> {
        self.post("/api/control", &serde_json::json!({ "command": command }))
    }

    pub fn play_track(&self, id: i64) -> Result<serde_json::Value> {
        self.post(
            "/api/control",
            &serde_json::json!({ "command": "play_track", "id": id }),
        )
    }

    pub fn play_tracks(&self, ids: &[i64], start_index: usize) -> Result<serde_json::Value> {
        self.post(
            "/api/control",
            &serde_json::json!({
                "command": "play_tracks",
                "ids": ids,
                "start_index": start_index,
            }),
        )
    }

    pub fn playback(&self) -> Result<PlaybackSnapshot> {
        self.get("/api/player")
    }

    pub fn library_snapshot(&self) -> Result<LibrarySnapshot> {
        #[derive(serde::Deserialize)]
        struct Page {
            items: Vec<Track>,
            next_offset: Option<usize>,
        }

        let library: serde_json::Value = self.get("/api/library")?;
        let playlists: Vec<Playlist> = self.get("/api/playlists").unwrap_or_default();
        let mixes: Vec<Mix> = self.get("/api/mixes").unwrap_or_default();
        let sources = self.library_sources().unwrap_or_default();
        let mut tracks = Vec::new();
        let mut offset = 0;
        loop {
            let page: Page = self.get(&format!(
                "/api/v2/tracks?limit=500&offset={offset}&sort=title"
            ))?;
            tracks.extend(page.items);
            match page.next_offset {
                Some(next) => offset = next,
                None => break,
            }
        }
        let counts = library
            .get("counts")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        Ok(LibrarySnapshot {
            tracks,
            playlists,
            mixes,
            sources,
            counts,
        })
    }

    pub fn rescan_and_library_snapshot(&self) -> Result<LibrarySnapshot> {
        let _: serde_json::Value = self.post(
            "/api/library/rescan",
            &serde_json::json!({ "mode": "changed" }),
        )?;

        // The rescan command is delivered to the render/runtime thread, so give
        // it a moment to leave the previous completed state before polling.
        std::thread::sleep(Duration::from_millis(100));
        let deadline = Instant::now() + Duration::from_secs(30 * 60);
        loop {
            let library: serde_json::Value = self.get("/api/library")?;
            let state = library
                .pointer("/scan/state")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("complete");
            if state == "complete" || state == "idle" {
                break;
            }
            if state == "error" || state == "failed" {
                let detail = library
                    .pointer("/scan/last_error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown scan error");
                anyhow::bail!("library scan failed: {detail}");
            }
            if Instant::now() >= deadline {
                anyhow::bail!("library scan did not finish within 30 minutes");
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        self.library_snapshot()
    }

    pub fn library_sources(&self) -> Result<Vec<LibrarySource>> {
        #[derive(serde::Deserialize)]
        struct SourceList {
            #[serde(default)]
            items: Vec<LibrarySource>,
        }
        Ok(self.get::<SourceList>("/api/library/sources")?.items)
    }

    pub fn add_reference_source(&self, path: &std::path::Path) -> Result<LibrarySource> {
        #[derive(serde::Deserialize)]
        struct SourceResponse {
            source: LibrarySource,
        }
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Referenced music");
        Ok(self
            .post::<_, SourceResponse>(
                "/api/library/sources",
                &serde_json::json!({
                    "kind": "reference",
                    "path": path,
                    "display_name": display_name
                }),
            )?
            .source)
    }

    pub fn update_library_source(
        &self,
        id: i64,
        display_name: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<LibrarySource> {
        #[derive(serde::Deserialize)]
        struct SourceResponse {
            source: LibrarySource,
        }
        Ok(self
            .patch::<_, SourceResponse>(
                &format!("/api/library/sources/{id}"),
                &serde_json::json!({
                    "display_name": display_name,
                    "enabled": enabled
                }),
            )?
            .source)
    }

    pub fn remove_library_source(&self, id: i64) -> Result<serde_json::Value> {
        self.delete(&format!("/api/library/sources/{id}"))
    }

    pub fn rescan_library_source(&self, id: i64) -> Result<serde_json::Value> {
        self.post(
            &format!("/api/library/sources/{id}/rescan"),
            &serde_json::json!({}),
        )
    }

    pub(crate) fn raw_client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, ensure_leading_slash(path))
    }
}

fn decode<T: DeserializeOwned>(
    response: Result<Response, reqwest::Error>,
    path: &str,
) -> Result<T> {
    let response = response.with_context(|| format!("requesting {path}"))?;
    let status = response.status();
    if !status.is_success() {
        let detail = response.text().unwrap_or_default();
        return Err(anyhow!(
            "{path} returned HTTP {}{}",
            status.as_u16(),
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {detail}")
            }
        ));
    }
    response
        .json()
        .with_context(|| format!("decoding response from {path}"))
}

fn ensure_leading_slash(path: &str) -> &str {
    if path.starts_with('/') {
        path
    } else {
        // All internal callers use absolute API paths. Treating this as a
        // programmer error avoids silently constructing a surprising URL.
        panic!("API paths must begin with '/'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_targets() {
        assert!(ApiClient::new("127.0.0.1:18080").is_err());
    }

    #[test]
    fn normalizes_trailing_slashes() {
        let client = ApiClient::new("http://127.0.0.1:18080/").unwrap();
        assert_eq!(
            client.url("/api/status"),
            "http://127.0.0.1:18080/api/status"
        );
    }

    #[test]
    fn decodes_wrapped_library_source_lists() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let responder = std::thread::spawn(move || {
            let request = server.recv().unwrap();
            assert_eq!(request.url(), "/api/library/sources");
            request
                .respond(
                    tiny_http::Response::from_string(
                        r#"{"items":[{"id":7,"kind":"reference","path":"/Music","display_name":"Archive","enabled":true,"available":false,"last_scan_at":null,"last_error":null}]}"#,
                    )
                    .with_header(
                        "content-type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                )
                .unwrap();
        });

        let sources = ApiClient::new(address).unwrap().library_sources().unwrap();
        responder.join().unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].display_name, "Archive");
        assert!(!sources[0].available);
    }
}
