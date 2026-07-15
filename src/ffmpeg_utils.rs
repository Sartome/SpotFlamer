use std::path::Path;
use tokio::process::Command;
use tracing::debug;

/// Returns the path to the local ffmpeg executable (next to the running binary).
pub fn get_ffmpeg_path() -> std::path::PathBuf {
    let exe_name = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
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

/// Checks that `ffmpeg` is available locally and returns its version string.
pub async fn check_ffmpeg() -> Result<String, String> {
    let exe = get_ffmpeg_path();
    if !exe.exists() {
        return Err(format!(
            "ffmpeg introuvable à {} — placez ffmpeg(.exe) à côté de l'exécutable SpotFlamer",
            exe.display()
        ));
    }

    let output = Command::new(&exe)
        .arg("-version")
        .output()
        .await
        .map_err(|e| format!("Impossible de lancer ffmpeg: {e}"))?;

    if !output.status.success() {
        return Err("ffmpeg -version a retourné une erreur".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version_line = stdout.lines().next().unwrap_or("ffmpeg (version inconnue)");
    Ok(version_line.to_string())
}

/// Converts an audio file to MP3 320 kbps.
pub async fn convert_to_mp3(input: &Path, output: &Path) -> Result<(), String> {
    let exe = get_ffmpeg_path();
    debug!("ffmpeg: {} → {}", input.display(), output.display());

    let status = Command::new(&exe)
        .args([
            "-y",
            "-i",
            &input.to_string_lossy(),
            "-vn",
            "-codec:a", "libmp3lame",
            "-b:a", "320k",
            "-map_metadata", "-1",
            &output.to_string_lossy().to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .await
        .map_err(|e| format!("Impossible de lancer ffmpeg: {e}"))?;

    if !status.success() {
        return Err("ffmpeg conversion échouée".into());
    }

    let _ = tokio::fs::remove_file(input).await;
    Ok(())
}
