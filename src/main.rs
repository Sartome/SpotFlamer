mod app;
mod config;
mod downloader;
mod ffmpeg_utils;
mod metadata;
mod spotify;
mod ui;
mod youtube;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SpotFlamer 🔥")
            .with_inner_size(egui::vec2(680.0, 520.0))
            .with_min_inner_size(egui::vec2(500.0, 380.0)),
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
