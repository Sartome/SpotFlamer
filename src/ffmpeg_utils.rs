use std::path::Path;
use tokio::process::Command;
use tracing::debug;

fn hidden_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut std_cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std_cmd.creation_flags(0x08000000);
    }
    Command::from(std_cmd)
}

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

    let output = hidden_command(&exe)
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

/// Converts an audio file to MP3.
pub async fn convert_to_mp3(input: &Path, output: &Path, force_quality: bool, audio_quality: &str) -> Result<(), String> {
    let exe = get_ffmpeg_path();
    debug!("ffmpeg: {} → {}", input.display(), output.display());

    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-vn".to_string(),
        "-codec:a".to_string(),
        "libmp3lame".to_string(),
    ];

    if force_quality {
        args.push("-b:a".to_string());
        args.push(format!("{}k", audio_quality));
    } else {
        // -q:a 0 means variable bitrate, preserving highest source quality natively
        args.push("-q:a".to_string());
        args.push("0".to_string());
    }

    args.push("-map_metadata".to_string());
    args.push("-1".to_string());
    args.push(output.to_string_lossy().to_string());

    let status = hidden_command(&exe)
        .args(&args)
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
