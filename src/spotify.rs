#![allow(dead_code)]

use regex::Regex;
use reqwest::Client;
use tracing::debug;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: u32,
    pub total_tracks: u32,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotifyLinkType {
    Track,
    Album,
    Playlist,
}

#[derive(Debug, Clone)]
pub struct SpotifyLink {
    pub link_type: SpotifyLinkType,
    pub id: String,
}

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

// ---------------------------------------------------------------------------
// URL parsing
// ---------------------------------------------------------------------------

pub fn parse_spotify_url(input: &str) -> Option<SpotifyLink> {
    let input = input.trim();
    let re = Regex::new(
        r"(?:open\.spotify\.com|spotify)[:/](track|album|playlist)[:/]([A-Za-z0-9]+)",
    )
    .ok()?;
    let caps = re.captures(input)?;
    let link_type = match &caps[1] {
        "track" => SpotifyLinkType::Track,
        "album" => SpotifyLinkType::Album,
        "playlist" => SpotifyLinkType::Playlist,
        _ => return None,
    };
    Some(SpotifyLink {
        link_type,
        id: caps[2].to_string(),
    })
}

// ---------------------------------------------------------------------------
// Public fetch functions (no auth — scrapes public HTML)
// ---------------------------------------------------------------------------

pub async fn fetch_track(id: &str) -> Result<TrackInfo, String> {
    let client = build_client()?;
    scrape_track(&client, id).await
}

pub async fn fetch_album_tracks(id: &str) -> Result<Vec<TrackInfo>, String> {
    let client = build_client()?;
    let html = fetch_page(&client, &format!("https://open.spotify.com/embed/album/{id}")).await?;

    let re = Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.*?)</script>"#).unwrap();
    let json_str = re.captures(&html).map(|c| html_decode(&c[1])).ok_or_else(|| "Impossible de trouver les données de l'album".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| format!("Parse JSON erreur: {e}"))?;
    let entity = v.pointer("/props/pageProps/state/data/entity").ok_or_else(|| "Données introuvables".to_string())?;

    let album_name = entity["title"].as_str().or(entity["name"].as_str()).unwrap_or("").to_string();
    let cover_url = entity["coverArt"]["sources"][0]["url"].as_str().map(|s| s.to_string());

    let track_list = entity["trackList"].as_array();
    
    if let Some(list) = track_list {
        let total = list.len() as u32;
        let mut tracks = Vec::with_capacity(list.len());
        for (i, t) in list.iter().enumerate() {
            if t["uri"].as_str().unwrap_or("").starts_with("spotify:track:") {
                let title = t["title"].as_str().unwrap_or("").to_string();
                let artist = t["subtitle"].as_str().unwrap_or("").to_string();
                let duration_ms = t["duration"].as_u64().unwrap_or(0);
                
                if !title.is_empty() {
                    tracks.push(TrackInfo {
                        title,
                        artist,
                        album: album_name.clone(),
                        track_number: (i + 1) as u32,
                        total_tracks: total,
                        duration_ms,
                        cover_url: cover_url.clone(),
                    });
                }
            }
        }
        if !tracks.is_empty() {
            return Ok(tracks);
        }
    }

    Err("Aucune piste valide trouvée dans cet album".into())
}

pub async fn fetch_playlist_tracks(id: &str) -> Result<Vec<TrackInfo>, String> {
    let client = build_client()?;
    let html = fetch_page(&client, &format!("https://open.spotify.com/embed/playlist/{id}")).await?;

    let re = Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.*?)</script>"#).unwrap();
    let json_str = re.captures(&html).map(|c| html_decode(&c[1])).ok_or_else(|| "Impossible de trouver les données de la playlist".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| format!("Parse JSON erreur: {e}"))?;
    let entity = v.pointer("/props/pageProps/state/data/entity").ok_or_else(|| "Données introuvables".to_string())?;

    let cover_url = entity["coverArt"]["sources"][0]["url"].as_str().map(|s| s.to_string());
    let playlist_name = entity["name"].as_str().unwrap_or("Playlist").to_string();

    let track_list = entity["trackList"].as_array();
    
    if let Some(list) = track_list {
        let total = list.len() as u32;
        let mut tracks = Vec::with_capacity(list.len());
        for (i, t) in list.iter().enumerate() {
            if t["uri"].as_str().unwrap_or("").starts_with("spotify:track:") {
                let title = t["title"].as_str().unwrap_or("").to_string();
                let artist = t["subtitle"].as_str().unwrap_or("").to_string();
                let duration_ms = t["duration"].as_u64().unwrap_or(0);
                
                if !title.is_empty() {
                    tracks.push(TrackInfo {
                        title,
                        artist,
                        album: playlist_name.clone(),
                        track_number: (i + 1) as u32,
                        total_tracks: total,
                        duration_ms,
                        cover_url: cover_url.clone(),
                    });
                }
            }
        }
        if !tracks.is_empty() {
            return Ok(tracks);
        }
    }

    Err("Aucune piste valide trouvée dans cette playlist".into())
}

// ---------------------------------------------------------------------------
// Internal scraping
// ---------------------------------------------------------------------------

fn build_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))
}

async fn fetch_page(client: &Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Requête Spotify échouée: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Spotify HTTP {}", resp.status()));
    }

    resp.text()
        .await
        .map_err(|e| format!("Lecture réponse échouée: {e}"))
}

async fn scrape_track(client: &Client, id: &str) -> Result<TrackInfo, String> {
    let url = format!("https://open.spotify.com/embed/track/{id}");
    let html = fetch_page(client, &url).await?;

    // Extract JSON state
    let re = Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.*?)</script>"#).unwrap();
    let json_str = re.captures(&html).map(|c| html_decode(&c[1])).ok_or_else(|| "Impossible de trouver les données Spotify".to_string())?;

    let v: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| format!("Parse JSON erreur: {e}"))?;

    let entity = v.pointer("/props/pageProps/state/data/entity").ok_or_else(|| "Données piste introuvables".to_string())?;

    let title = entity["title"].as_str().or(entity["name"].as_str()).unwrap_or("").to_string();
    
    let artist = entity["artists"][0]["name"].as_str().unwrap_or("").to_string();
    let duration_ms = entity["duration"].as_u64().unwrap_or(0);
    
    let cover_url = entity["coverArt"]["sources"][0]["url"].as_str().map(|s| s.to_string());

    if title.is_empty() {
        return Err(format!("Impossible d'extraire le titre de la piste {id}"));
    }

    Ok(TrackInfo {
        title,
        artist,
        album: String::new(), // Embed ne fournit pas l'album au premier niveau pour la track
        track_number: 0,
        total_tracks: 0,
        duration_ms,
        cover_url,
    })
}

// ---------------------------------------------------------------------------
// HTML parsing helpers
// ---------------------------------------------------------------------------

/// Parses the `<title>` tag to extract song title and artist.
/// Handles formats:
///   - "Song Name - song and lyrics by Artist Name | Spotify"
///   - "Song Name - song by Artist Name | Spotify"
///   - "Song Name | Spotify"
fn parse_title_tag(html: &str) -> (String, String) {
    let re = Regex::new(r"<title[^>]*>(.*?)</title>").ok();
    let raw = match re.and_then(|r| r.captures(html)) {
        Some(c) => html_decode(&c[1]),
        None => return (String::new(), String::new()),
    };

    // Strip " | Spotify" suffix
    let content = raw
        .rsplit_once(" | ")
        .or_else(|| raw.rsplit_once(" · "))
        .map(|(left, _)| left.trim())
        .unwrap_or(raw.trim());

    // Try to split on " - song" pattern
    if let Some(idx) = content.to_lowercase().find(" - song") {
        let title = content[..idx].trim().to_string();
        let rest = &content[idx..];
        // Find "by " in the rest
        let artist = rest
            .to_lowercase()
            .find(" by ")
            .map(|bi| rest[(bi + 4)..].trim().to_string())
            .unwrap_or_default();
        return (title, artist);
    }

    // Fallback: try "Title - Artist" format (less common)
    if let Some((title, artist)) = content.split_once(" - ") {
        return (title.trim().to_string(), artist.trim().to_string());
    }

    // Last resort: entire content is the title
    (content.to_string(), String::new())
}

/// Extracts a `<meta property="X" content="Y">` value, handling both attribute orders.
fn extract_meta(html: &str, property: &str) -> Option<String> {
    // property="X" content="Y"
    let p1 = format!(
        r#"<meta\s+property\s*=\s*"{property}"\s+content\s*=\s*"([^"]*)""#
    );
    // content="Y" property="X"
    let p2 = format!(
        r#"<meta\s+content\s*=\s*"([^"]*)"\s+property\s*=\s*"{property}""#
    );

    for pattern in [&p1, &p2] {
        if let Some(re) = Regex::new(pattern).ok() {
            if let Some(caps) = re.captures(html) {
                return Some(html_decode(&caps[1]));
            }
        }
    }
    None
}

/// Tries to extract the album name from the page's description meta tag.
/// Description is often: "Artist · Song · Album · 2023" or similar.
fn extract_album_from_description(html: &str) -> Option<String> {
    let desc = extract_meta(html, "og:description")
        .or_else(|| {
            // Try name="description" variant
            let re = Regex::new(r#"<meta\s+name\s*=\s*"description"\s+content\s*=\s*"([^"]*)""#).ok()?;
            re.captures(html).map(|c| html_decode(&c[1]))
        })?;

    // Description formats:
    //   "Artist · Song · Album · Year"
    //   "Listen to X on Spotify. Artist · Album · Year"
    let parts: Vec<&str> = desc.split('·').map(|s| s.trim()).collect();

    // If we have at least 3 parts, the album is typically the second-to-last before the year
    if parts.len() >= 3 {
        // Skip the last part if it looks like a year (4 digits)
        let last = parts.last().unwrap_or(&"");
        let album_idx = if last.trim().len() == 4 && last.trim().parse::<u32>().is_ok() {
            parts.len() - 2
        } else {
            parts.len() - 1
        };
        if album_idx > 0 {
            let candidate = parts[album_idx].trim();
            // Avoid returning "Song" or "Single" as album name
            if candidate != "Song" && candidate != "Single" && !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

/// Extracts Spotify track IDs from HTML (from meta tags or href links).
fn extract_track_ids(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // 1. Try music:song meta tags: <meta property="music:song" content="https://open.spotify.com/track/XXX">
    if let Ok(re) = Regex::new(r#"music:song"[^>]*content\s*=\s*"[^"]*?/track/([A-Za-z0-9]{22})"#) {
        for caps in re.captures_iter(html) {
            let id = caps[1].to_string();
            if seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    }

    // 2. Fallback: find track IDs in href links
    if ids.is_empty() {
        if let Ok(re) = Regex::new(r#"href\s*=\s*"/track/([A-Za-z0-9]{22})"#) {
            for caps in re.captures_iter(html) {
                let id = caps[1].to_string();
                if seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
        }
    }

    // 3. Last resort: any track ID pattern in the HTML
    if ids.is_empty() {
        if let Ok(re) = Regex::new(r#"spotify[:/]track[:/]([A-Za-z0-9]{22})"#) {
            for caps in re.captures_iter(html) {
                let id = caps[1].to_string();
                if seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
        }
    }

    debug!("Found {} track IDs in HTML", ids.len());
    ids
}

/// Decodes basic HTML entities.
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
}
