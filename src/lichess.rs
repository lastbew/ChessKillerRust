//! lichess.rs - the Lichess Board API using blocking reqwest.

use std::io::{BufRead, BufReader};

use anyhow::{bail, Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;

const BASE: &str = "https://lichess.org";

pub struct LichessClient {
    http: Client,
}

impl LichessClient {
    pub fn new(token: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        let auth: HeaderValue = format!("Bearer {token}")
            .parse()
            .context("invalid characters in token")?;
        headers.insert(AUTHORIZATION, auth);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let http = Client::builder().default_headers(headers).build()?;
        Ok(LichessClient { http })
    }

    pub fn get_account(&self) -> Result<Value> {
        let resp = self.http.get(format!("{BASE}/api/account")).send()?;
        if !resp.status().is_success() {
            bail!(
                "login failed (HTTP {}) - check LICHESS_TOKEN and its board:play scope",
                resp.status()
            );
        }
        Ok(resp.json()?)
    }

    pub fn get_ongoing_games(&self) -> Result<Vec<Value>> {
        let resp = self.http.get(format!("{BASE}/api/account/playing")).send()?;
        if !resp.status().is_success() {
            bail!("could not list games (HTTP {})", resp.status());
        }
        let json: Value = resp.json()?;
        Ok(json
            .get("nowPlaying")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    pub fn create_challenge(&self, username: &str, minutes: u32, increment: u32) -> Result<()> {
        let limit = (minutes * 60).to_string();
        let inc = increment.to_string();
        let form = [
            ("rated", "false"),
            ("clock.limit", limit.as_str()),
            ("clock.increment", inc.as_str()),
            ("color", "random"),
        ];
        let resp = self
            .http
            .post(format!("{BASE}/api/challenge/{username}"))
            .form(&form)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("challenge failed (HTTP {status}): {body}");
        }
        Ok(())
    }

    pub fn wait_for_game(&self, timeout_secs: u64) -> Result<Option<String>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            if let Some(game) = self.get_ongoing_games()?.into_iter().next() {
                if let Some(id) = game.get("gameId").and_then(Value::as_str) {
                    return Ok(Some(id.to_string()));
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        Ok(None)
    }

    /// Open a game's event stream and return an iterator of NDJSON values.
    pub fn stream_game(&self, game_id: &str) -> Result<NdjsonIter> {
        let resp = self
            .http
            .get(format!("{BASE}/api/board/game/stream/{game_id}"))
            .send()?;
        if !resp.status().is_success() {
            bail!("could not open game stream (HTTP {})", resp.status());
        }
        Ok(NdjsonIter { reader: BufReader::new(resp) })
    }

    pub fn make_move(&self, game_id: &str, uci: &str) -> Result<bool> {
        let resp = self
            .http
            .post(format!("{BASE}/api/board/game/{game_id}/move/{uci}"))
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("move {uci} rejected (HTTP {status}): {body}");
        }
        let json: Value = resp.json()?;
        Ok(json.get("ok").and_then(Value::as_bool).unwrap_or(false))
    }

    #[allow(dead_code)]
    pub fn resign(&self, game_id: &str) -> Result<()> {
        let _ = self
            .http
            .post(format!("{BASE}/api/board/game/{game_id}/resign"))
            .send()?;
        Ok(())
    }
}

/// Turns the newline-delimited JSON stream into an iterator of `serde_json::Value`.
pub struct NdjsonIter {
    reader: BufReader<Response>,
}

impl Iterator for NdjsonIter {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue; // heartbeat
                    }
                    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                        return Some(value);
                    }
                }
                Err(_) => return None,
            }
        }
    }
}
