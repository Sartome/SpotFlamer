use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = ".spotflamer";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub output_dir: PathBuf,
    pub add_track_number: bool,
    #[serde(default = "default_true")]
    pub create_subfolder: bool,
    #[serde(default = "default_false")]
    pub force_quality: bool,
    #[serde(default = "default_quality")]
    pub audio_quality: String,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_quality() -> String { "320".to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        let output_dir = dirs::audio_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("SpotFlamer");

        Self {
            output_dir,
            add_track_number: true,
            create_subfolder: true,
            force_quality: false,
            audio_quality: "320".to_string(),
        }
    }
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME))
    }

    pub fn load() -> Self {
        Self::config_path()
            .and_then(|p| std::fs::read_to_string(&p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}
