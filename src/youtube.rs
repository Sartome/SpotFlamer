#![allow(dead_code)]

use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, warn};

fn hidden_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut std_cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std_cmd.creation_flags(0x08000000);
    }
    Command::from(std_cmd)
}

// ---------------------------------------------------------------------------
// Local executable path
// ---------------------------------------------------------------------------

/// Returns the path to the local yt-dlp executable (next to the running binary).
pub fn get_ytdlp_path() -> std::path::PathBuf {
    let exe_name = if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };

    // 1. Try executable directory (for released app)
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push(exe_name);
        if path.exists() {
            return path;
        }
    }

    // 2. Try current working directory (for cargo run)
    if let Ok(mut path) = std::env::current_dir() {
        path.push(exe_name);
        if path.exists() {
            return path;
        }
    }

    // Fallback: expect it in PATH
    std::path::PathBuf::from(exe_name)
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct YtdlpEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    uploader: Option<String>,
    #[serde(default)]
    webpage_url: Option<String>,
    #[serde(default)]
    channel_is_verified: Option<bool>,
}

#[derive(Debug)]
pub struct YtDownloadResult {
    pub file_path: PathBuf,
    pub video_title: String,
}

// ---------------------------------------------------------------------------
// URL helpers
// ---------------------------------------------------------------------------

pub fn parse_youtube_url(input: &str) -> Option<String> {
    let input = input.trim();
    let re = regex::Regex::new(
        r"(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/embed/|youtube\.com/v/)([A-Za-z0-9_-]{11})",
    )
    .ok()?;
    re.captures(input).map(|c| c[1].to_string())
}

// ---------------------------------------------------------------------------
// Search + score + download
// ---------------------------------------------------------------------------

pub async fn search_and_download(
    artist: &str,
    title: &str,
    expected_duration_secs: f64,
    output_dir: &Path,
) -> Result<YtDownloadResult, String> {
    let exe = get_ytdlp_path();

    let queries = [
        format!("{artist} - {title} Official Audio"),
        format!("{artist} - {title} Topic"),
        format!("{artist} {title}"),
    ];

    let mut all_entries: Vec<YtdlpEntry> = Vec::new();

    for query in &queries {
        let search_arg = format!("ytsearch5:{query}");
        debug!("yt-dlp search: {search_arg}");

        let output = hidden_command(&exe)
            .args(["--dump-json", "--flat-playlist", "--no-warnings", "--extractor-args", "youtube:player_client=default,ios", &search_arg])
            .output()
            .await
            .map_err(|e| format!("Impossible de lancer yt-dlp ({}): {e}", exe.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("yt-dlp search failed for '{query}': {stderr}");
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Ok(entry) = serde_json::from_str::<YtdlpEntry>(line) {
                if !entry.id.is_empty() && !all_entries.iter().any(|e| e.id == entry.id) {
                    all_entries.push(entry);
                }
            }
        }

        if !all_entries.is_empty() {
            break;
        }
    }

    if all_entries.is_empty() {
        return Err(format!(
            "Aucun résultat YouTube pour \"{artist} - {title}\""
        ));
    }

    // Score each result
    let mut best: Option<(i32, &YtdlpEntry)> = None;
    for entry in &all_entries {
        let score = score_entry(entry, artist, title, expected_duration_secs);
        debug!(
            "  score={score:+4} | {} | dur={:?}s | {}",
            entry.id, entry.duration, entry.title
        );
        if best.is_none() || score > best.as_ref().map(|b| b.0).unwrap_or(i32::MIN) {
            best = Some((score, entry));
        }
    }

    let (best_score, chosen) = best.ok_or("Aucun résultat à scorer")?;
    if best_score < -30 {
        return Err(format!(
            "Pas de bon match YouTube pour \"{artist} - {title}\" (score: {best_score})"
        ));
    }

    let video_url = chosen
        .webpage_url
        .clone()
        .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", chosen.id));

    debug!("Selected: {} (score {best_score})", chosen.title);

    download_audio(&exe, &video_url, output_dir, &chosen.id).await?;
    let downloaded = find_downloaded_file(output_dir, &chosen.id)?;

    Ok(YtDownloadResult {
        file_path: downloaded,
        video_title: chosen.title.clone(),
    })
}

pub async fn download_direct(
    video_url: &str,
    output_dir: &Path,
) -> Result<YtDownloadResult, String> {
    let exe = get_ytdlp_path();

    let output = hidden_command(&exe)
        .args(["--dump-json", "--no-warnings", "--extractor-args", "youtube:player_client=default,ios", video_url])
        .output()
        .await
        .map_err(|e| format!("Impossible de lancer yt-dlp: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp metadata échoué: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entry: YtdlpEntry = serde_json::from_str(stdout.lines().next().unwrap_or("{}"))
        .map_err(|e| format!("Parse yt-dlp JSON: {e}"))?;

    download_audio(&exe, video_url, output_dir, &entry.id).await?;
    let file_path = find_downloaded_file(output_dir, &entry.id)?;

    Ok(YtDownloadResult {
        file_path,
        video_title: entry.title,
    })
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn score_entry(entry: &YtdlpEntry, artist: &str, title: &str, expected_dur: f64) -> i32 {
    let mut score: i32 = 0;
    let t = entry.title.to_lowercase();
    let artist_lower = artist.to_lowercase();
    let title_lower = title.to_lowercase();

    if t.contains("official audio") { score += 15; }
    if t.contains("topic") { score += 12; }
    if t.contains(&artist_lower) && t.contains(&title_lower) { score += 10; }
    if let Some(ref ch) = entry.channel {
        let ch_lower = ch.to_lowercase();
        if ch_lower.contains("topic") || ch_lower.contains(&artist_lower) { score += 8; }
    }
    if entry.channel_is_verified == Some(true) { score += 5; }

    let bad_words = [
        "live", "concert", "cover", "remix", "karaoke", "parody",
        "parodie", "reprise", "acoustic version", "slowed", "sped up",
        "8d audio", "nightcore",
    ];
    for w in &bad_words {
        if t.contains(w) { score -= 25; }
    }
    if t.contains("lyric") { score -= 5; }

    if expected_dur > 0.0 {
        if let Some(dur) = entry.duration {
            let diff = (dur - expected_dur).abs();
            if diff <= 3.0 { score += 15; }
            else if diff <= 7.0 { score += 8; }
            else if diff <= 12.0 { /* neutral */ }
            else if diff <= 30.0 { score -= 15; }
            else { score -= 40; }
        }
    }

    score
}

async fn download_audio(exe: &Path, video_url: &str, output_dir: &Path, video_id: &str) -> Result<(), String> {
    let output_template = output_dir.join(format!("{video_id}.%(ext)s"));

    let output = hidden_command(exe)
        .args([
            "-f", "bestaudio",
            "--no-playlist",
            "--no-warnings",
            "--extractor-args", "youtube:player_client=default,ios",
            "-o", &output_template.to_string_lossy(),
            video_url,
        ])
        .output()
        .await
        .map_err(|e| format!("yt-dlp exécution échouée: {e}"))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp erreur: {}", err_msg.trim()));
    }
    Ok(())
}

fn find_downloaded_file(dir: &Path, video_id: &str) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Impossible de lire le dossier: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(video_id) && entry.path().is_file() {
            return Ok(entry.path());
        }
    }
    Err(format!(
        "Fichier téléchargé introuvable pour {video_id} dans {}",
        dir.display()
    ))
}
