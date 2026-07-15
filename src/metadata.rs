use crate::spotify::TrackInfo;
use id3::frame::{Picture, PictureType};
use id3::{Tag, TagLike, Version};
use reqwest::Client;
use std::path::Path;
use tracing::debug;

/// Downloads cover art and applies ID3v2.4 tags to an MP3 file.
pub async fn apply_tags(mp3_path: &Path, info: &TrackInfo) -> Result<(), String> {
    let mut tag = Tag::new();

    tag.set_title(&info.title);
    tag.set_artist(&info.artist);
    tag.set_album(&info.album);
    tag.set_track(info.track_number);
    tag.set_total_tracks(info.total_tracks);

    // Download and embed cover art
    if let Some(ref cover_url) = info.cover_url {
        match download_cover(cover_url).await {
            Ok(data) => {
                let picture = Picture {
                    mime_type: "image/jpeg".to_string(),
                    picture_type: PictureType::CoverFront,
                    description: String::new(),
                    data,
                };
                tag.add_frame(picture);
                debug!("Cover art embedded");
            }
            Err(e) => {
                // Non-fatal: continue without cover art
                tracing::warn!("Could not download cover art: {e}");
            }
        }
    }

    tag.write_to_path(mp3_path, Version::Id3v24)
        .map_err(|e| format!("Failed to write ID3 tags: {e}"))?;

    debug!("ID3 tags applied to {}", mp3_path.display());
    Ok(())
}

async fn download_cover(url: &str) -> Result<Vec<u8>, String> {
    let client = Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Cover download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Cover download HTTP {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Cover read failed: {e}"))?;

    Ok(bytes.to_vec())
}
