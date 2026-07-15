use crate::config::AppConfig;
use crate::spotify::{self, SpotifyLinkType, TrackInfo};
use crate::youtube;
use crate::{ffmpeg_utils, metadata};
use tokio::sync::mpsc;
use tracing::{error, info};

// ---------------------------------------------------------------------------
// Types shared between UI and backend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QueueItem {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Searching,
    Downloading,
    Converting,
    Tagging,
    Done,
    Error(String),
}

impl std::fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "En attente"),
            Self::Searching => write!(f, "Recherche…"),
            Self::Downloading => write!(f, "Téléchargement…"),
            Self::Converting => write!(f, "Conversion MP3…"),
            Self::Tagging => write!(f, "Métadonnées…"),
            Self::Done => write!(f, "Terminé ✓"),
            Self::Error(e) => write!(f, "Erreur: {e}"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ItemMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
}

#[derive(Debug, Clone)]
pub struct StatusUpdate {
    pub item_id: u64,
    pub status: DownloadStatus,
    pub meta: Option<ItemMeta>,
}

#[derive(Debug)]
pub enum DownloadCommand {
    ProcessInput { input: String, config: AppConfig },
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

pub fn spawn_worker(
    ctx: egui::Context,
) -> (
    mpsc::UnboundedSender<DownloadCommand>,
    mpsc::UnboundedReceiver<StatusUpdate>,
) {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<DownloadCommand>();
    let (status_tx, status_rx) = mpsc::unbounded_channel::<StatusUpdate>();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(3));

    tokio::spawn(async move {
        let id_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1));

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                DownloadCommand::ProcessInput { input, config } => {
                    let status_tx = status_tx.clone();
                    let semaphore = semaphore.clone();
                    let id_counter = id_counter.clone();
                    let ctx = ctx.clone();

                    tokio::spawn(async move {
                        process_input(input, config, status_tx, semaphore, id_counter, ctx).await;
                    });
                }
            }
        }
    });

    (cmd_tx, status_rx)
}

// ---------------------------------------------------------------------------
// Input processing — no auth needed
// ---------------------------------------------------------------------------

async fn process_input(
    input: String,
    config: AppConfig,
    tx: mpsc::UnboundedSender<StatusUpdate>,
    semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    id_counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
    ctx: egui::Context,
) {
    let input = input.trim().to_string();

    // --- Spotify link ---
    if let Some(link) = spotify::parse_spotify_url(&input) {
        // Scrape metadata directly from public Spotify pages (no API key needed)
        let tracks = match link.link_type {
            SpotifyLinkType::Track => match spotify::fetch_track(&link.id).await {
                Ok(t) => vec![t],
                Err(e) => { emit_error(&tx, &id_counter, &ctx, &e); return; }
            },
            SpotifyLinkType::Album => match spotify::fetch_album_tracks(&link.id).await {
                Ok(t) => t,
                Err(e) => { emit_error(&tx, &id_counter, &ctx, &e); return; }
            },
            SpotifyLinkType::Playlist => match spotify::fetch_playlist_tracks(&link.id).await {
                Ok(t) => t,
                Err(e) => { emit_error(&tx, &id_counter, &ctx, &e); return; }
            },
        };

        for track in tracks {
            let item_id = next_id(&id_counter);
            let _ = tx.send(StatusUpdate {
                item_id,
                status: DownloadStatus::Queued,
                meta: Some(ItemMeta {
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                }),
            });
            ctx.request_repaint();

            let tx2 = tx.clone();
            let sem = semaphore.clone();
            let cfg = config.clone();
            let ctx2 = ctx.clone();
            tokio::spawn(async move {
                download_spotify_track(item_id, track, cfg, tx2, sem, ctx2).await;
            });
        }
        return;
    }

    // --- YouTube direct link ---
    if youtube::parse_youtube_url(&input).is_some() {
        let item_id = next_id(&id_counter);
        let _ = tx.send(StatusUpdate {
            item_id,
            status: DownloadStatus::Queued,
            meta: Some(ItemMeta {
                title: "YouTube video".into(),
                artist: String::new(),
                album: String::new(),
            }),
        });
        ctx.request_repaint();

        let tx2 = tx.clone();
        let sem = semaphore.clone();
        let cfg = config.clone();
        let ctx2 = ctx.clone();
        tokio::spawn(async move {
            download_youtube_direct(item_id, &input, cfg, tx2, sem, ctx2).await;
        });
        return;
    }

    // --- Unknown input ---
    emit_error(&tx, &id_counter, &ctx, "Lien non reconnu. Collez un lien Spotify ou YouTube.");
}

// ---------------------------------------------------------------------------
// Spotify pipeline
// ---------------------------------------------------------------------------

async fn download_spotify_track(
    item_id: u64,
    track: TrackInfo,
    config: AppConfig,
    tx: mpsc::UnboundedSender<StatusUpdate>,
    semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    ctx: egui::Context,
) {
    let _permit = match semaphore.acquire().await {
        Ok(p) => p,
        Err(_) => return,
    };

    let send = |status: DownloadStatus| {
        let _ = tx.send(StatusUpdate { item_id, status, meta: None });
        ctx.request_repaint();
    };

    let _ = tokio::fs::create_dir_all(&config.output_dir).await;

    send(DownloadStatus::Searching);
    let expected_secs = track.duration_ms as f64 / 1000.0;
    let yt_result = match youtube::search_and_download(
        &track.artist,
        &track.title,
        expected_secs,
        &config.output_dir,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Search failed for {}: {e}", track.title);
            send(DownloadStatus::Error(e));
            return;
        }
    };

    send(DownloadStatus::Converting);
    let mp3_filename = build_filename(&track, config.add_track_number);
    let mp3_path = config.output_dir.join(&mp3_filename);
    if let Err(e) = ffmpeg_utils::convert_to_mp3(&yt_result.file_path, &mp3_path).await {
        error!("Convert failed for {}: {e}", track.title);
        send(DownloadStatus::Error(e));
        return;
    }

    send(DownloadStatus::Tagging);
    if let Err(e) = metadata::apply_tags(&mp3_path, &track).await {
        error!("Tagging failed for {}: {e}", track.title);
        send(DownloadStatus::Error(e));
        return;
    }

    info!("✓ {} → {}", track.title, mp3_path.display());
    send(DownloadStatus::Done);
}

// ---------------------------------------------------------------------------
// YouTube direct pipeline
// ---------------------------------------------------------------------------

async fn download_youtube_direct(
    item_id: u64,
    url: &str,
    config: AppConfig,
    tx: mpsc::UnboundedSender<StatusUpdate>,
    semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    ctx: egui::Context,
) {
    let _permit = match semaphore.acquire().await {
        Ok(p) => p,
        Err(_) => return,
    };

    let send = |status: DownloadStatus| {
        let _ = tx.send(StatusUpdate { item_id, status, meta: None });
        ctx.request_repaint();
    };

    let _ = tokio::fs::create_dir_all(&config.output_dir).await;

    send(DownloadStatus::Downloading);
    let yt_result = match youtube::download_direct(url, &config.output_dir).await {
        Ok(r) => r,
        Err(e) => {
            send(DownloadStatus::Error(e));
            return;
        }
    };

    let _ = tx.send(StatusUpdate {
        item_id,
        status: DownloadStatus::Converting,
        meta: Some(ItemMeta {
            title: yt_result.video_title.clone(),
            artist: String::new(),
            album: String::new(),
        }),
    });
    ctx.request_repaint();

    let safe_title = sanitize_filename(&yt_result.video_title);
    let mp3_path = config.output_dir.join(format!("{safe_title}.mp3"));
    if let Err(e) = ffmpeg_utils::convert_to_mp3(&yt_result.file_path, &mp3_path).await {
        send(DownloadStatus::Error(e));
        return;
    }

    send(DownloadStatus::Tagging);
    let info = TrackInfo {
        title: yt_result.video_title,
        artist: String::new(),
        album: String::new(),
        track_number: 0,
        total_tracks: 0,
        duration_ms: 0,
        cover_url: None,
    };
    let _ = metadata::apply_tags(&mp3_path, &info).await;

    info!("✓ YouTube direct → {}", mp3_path.display());
    send(DownloadStatus::Done);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn next_id(counter: &std::sync::atomic::AtomicU64) -> u64 {
    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn emit_error(
    tx: &mpsc::UnboundedSender<StatusUpdate>,
    id_counter: &std::sync::atomic::AtomicU64,
    ctx: &egui::Context,
    msg: &str,
) {
    let id = next_id(id_counter);
    let _ = tx.send(StatusUpdate {
        item_id: id,
        status: DownloadStatus::Error(msg.to_string()),
        meta: Some(ItemMeta {
            title: "Erreur".into(),
            artist: String::new(),
            album: String::new(),
        }),
    });
    ctx.request_repaint();
}

fn build_filename(track: &TrackInfo, add_track_number: bool) -> String {
    let safe_title = sanitize_filename(&track.title);
    let safe_artist = sanitize_filename(&track.artist);

    if add_track_number && track.track_number > 0 {
        format!("{:02} - {} - {}.mp3", track.track_number, safe_artist, safe_title)
    } else if !safe_artist.is_empty() {
        format!("{} - {}.mp3", safe_artist, safe_title)
    } else {
        format!("{}.mp3", safe_title)
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

impl QueueItem {
    pub fn new(id: u64, title: String, artist: String, album: String) -> Self {
        Self {
            id,
            title,
            artist,
            album,
            status: DownloadStatus::Queued,
        }
    }

    pub fn status_color(&self) -> egui::Color32 {
        match &self.status {
            DownloadStatus::Queued => egui::Color32::from_rgb(150, 150, 150),
            DownloadStatus::Searching => egui::Color32::from_rgb(100, 160, 255),
            DownloadStatus::Downloading => egui::Color32::from_rgb(80, 180, 255),
            DownloadStatus::Converting => egui::Color32::from_rgb(255, 180, 50),
            DownloadStatus::Tagging => egui::Color32::from_rgb(200, 140, 255),
            DownloadStatus::Done => egui::Color32::from_rgb(80, 220, 120),
            DownloadStatus::Error(_) => egui::Color32::from_rgb(255, 80, 80),
        }
    }
}
