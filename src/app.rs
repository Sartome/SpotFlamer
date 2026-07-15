use crate::config::AppConfig;
use crate::downloader::{
    DownloadCommand, DownloadStatus, QueueItem, StatusUpdate,
};
use crate::ui;
use tokio::sync::mpsc;

pub struct SpotFlamerApp {
    config: AppConfig,
    input_text: String,
    settings_open: bool,
    queue: Vec<QueueItem>,
    cmd_tx: mpsc::UnboundedSender<DownloadCommand>,
    status_rx: mpsc::UnboundedReceiver<StatusUpdate>,
    config_dirty: bool,
}

impl SpotFlamerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        let config = AppConfig::load();

        let (cmd_tx, status_rx) =
            crate::downloader::spawn_worker(cc.egui_ctx.clone());

        Self {
            config,
            input_text: String::new(),
            settings_open: false,
            queue: Vec::new(),
            cmd_tx,
            status_rx,
            config_dirty: false,
        }
    }

    /// Drain all pending status updates from the worker.
    fn poll_updates(&mut self) {
        while let Ok(update) = self.status_rx.try_recv() {
            // If this update carries metadata, create a new QueueItem
            if let Some(ref meta) = update.meta {
                if let Some(existing) = self.queue.iter_mut().find(|q| q.id == update.item_id) {
                    // Update existing item (e.g., YouTube direct title update)
                    if !meta.title.is_empty() {
                        existing.title = meta.title.clone();
                    }
                    if !meta.artist.is_empty() {
                        existing.artist = meta.artist.clone();
                    }
                    existing.status = update.status;
                } else {
                    // New item
                    let mut item = QueueItem::new(
                        update.item_id,
                        meta.title.clone(),
                        meta.artist.clone(),
                        meta.album.clone(),
                    );
                    item.status = update.status;
                    self.queue.push(item);
                }
            } else if let Some(item) = self.queue.iter_mut().find(|q| q.id == update.item_id) {
                item.status = update.status;
            }
        }
    }
}

impl eframe::App for SpotFlamerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_updates();

        let mut should_submit = false;
        let mut should_open_folder = false;
        let mut should_browse = false;
        let mut should_clear_done = false;

        ui::draw_ui(
            ctx,
            &mut self.input_text,
            &mut self.config,
            &self.queue,
            &mut self.settings_open,
            &mut || should_submit = true,
            &mut || should_open_folder = true,
            &mut || should_browse = true,
            &mut || should_clear_done = true,
        );

        // Handle deferred actions
        if should_submit {
            let input = self.input_text.trim().to_string();
            if !input.is_empty() {
                let _ = self.cmd_tx.send(DownloadCommand::ProcessInput {
                    input,
                    config: self.config.clone(),
                });
                self.input_text.clear();
            }
        }

        if should_open_folder {
            let _ = open::that(&self.config.output_dir);
        }

        if should_browse {
            if let Some(path) = rfd::FileDialog::new()
                .set_directory(&self.config.output_dir)
                .pick_folder()
            {
                self.config.output_dir = path;
                self.config_dirty = true;
            }
        }

        if should_clear_done {
            self.queue
                .retain(|q| !matches!(q.status, DownloadStatus::Done | DownloadStatus::Error(_)));
        }

        // Auto-save config when settings panel closes
        if self.config_dirty || self.settings_open {
            // Save periodically while settings are open
            self.config.save();
            self.config_dirty = false;
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.config.save();
    }
}
