#![windows_subsystem = "windows"]
mod app;
mod config;
mod downloader;
mod ffmpeg_utils;
mod metadata;
mod spotify;
mod ui;
mod youtube;
mod updater;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Start background update checker
    updater::spawn_update_checker();

    // Build tokio runtime
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    // Check ffmpeg availability at startup
    rt.block_on(async {
        match ffmpeg_utils::check_ffmpeg().await {
            Ok(version) => tracing::info!("ffmpeg found: {version}"),
            Err(e) => tracing::warn!("{e}"),
        }
    });

    let icon_data = match image::load_from_memory(include_bytes!("../spotflamer.png")) {
        Ok(img) => {
            let img = img.into_rgba8();
            let (width, height) = img.dimensions();
            let rgba = img.into_raw();
            Some(egui::IconData {
                rgba,
                width,
                height,
            })
        }
        Err(e) => {
            tracing::warn!("Failed to load icon: {e}");
            None
        }
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("SpotFlamer 🔥")
        .with_inner_size(egui::vec2(680.0, 520.0))
        .with_min_inner_size(egui::vec2(500.0, 380.0));

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "SpotFlamer",
        options,
        Box::new(|cc| Ok(Box::new(app::SpotFlamerApp::new(cc)))),
    ) {
        tracing::error!("eframe error: {e}");
    }
}
