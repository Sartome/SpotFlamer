use crate::config::AppConfig;
use crate::downloader::{DownloadStatus, QueueItem};
use egui::{vec2, Align, Color32, CornerRadius, Layout, RichText, Stroke, Vec2};

// ---------------------------------------------------------------------------
// Color palette — flame / amber theme
// ---------------------------------------------------------------------------

const BG_DARK: Color32 = Color32::from_rgb(18, 18, 24);
const BG_PANEL: Color32 = Color32::from_rgb(26, 26, 35);
const BG_CARD: Color32 = Color32::from_rgb(34, 34, 46);
const ACCENT: Color32 = Color32::from_rgb(255, 140, 50); // orange
#[allow(dead_code)]
const ACCENT_HOVER: Color32 = Color32::from_rgb(255, 170, 80);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 245);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 175);
const BORDER: Color32 = Color32::from_rgb(50, 50, 65);

// ---------------------------------------------------------------------------
// Public draw functions
// ---------------------------------------------------------------------------

pub fn draw_ui(
    ctx: &egui::Context,
    input_text: &mut String,
    config: &mut AppConfig,
    queue: &[QueueItem],
    settings_open: &mut bool,
    on_submit: &mut dyn FnMut(),
    on_open_folder: &mut dyn FnMut(),
    on_browse_folder: &mut dyn FnMut(),
    on_clear_done: &mut dyn FnMut(),
) {
    apply_theme(ctx);

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(BG_DARK).inner_margin(0.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

            // Header
            draw_header(ui, settings_open);

            ui.add_space(8.0);

            // Search bar
            draw_search_bar(ui, input_text, on_submit);

            ui.add_space(12.0);

            // Settings panel (collapsible)
            if *settings_open {
                draw_settings(ui, config, on_browse_folder);
                ui.add_space(12.0);
            }

            // Queue header
            draw_queue_header(ui, queue, on_open_folder, on_clear_done);

            ui.add_space(6.0);

            // Queue items
            draw_queue(ui, queue);
        });
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_DARK;
    visuals.window_fill = BG_PANEL;
    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.inactive.bg_fill = BG_CARD;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 45, 60);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.3);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    ctx.set_visuals(visuals);
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn draw_header(ui: &mut egui::Ui, settings_open: &mut bool) {
    egui::Frame::new()
        .fill(BG_PANEL)
        .inner_margin(egui::Margin::symmetric(20, 14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🔥 SpotFlamer")
                        .size(22.0)
                        .color(ACCENT)
                        .strong(),
                );
                ui.label(
                    RichText::new("Music Downloader")
                        .size(13.0)
                        .color(TEXT_SECONDARY),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let gear_text = if *settings_open { "⚙ Fermer" } else { "⚙ Paramètres" };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(gear_text).size(13.0).color(TEXT_SECONDARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        *settings_open = !*settings_open;
                    }
                });
            });
        });
}

// ---------------------------------------------------------------------------
// Search bar
// ---------------------------------------------------------------------------

fn draw_search_bar(ui: &mut egui::Ui, input_text: &mut String, on_submit: &mut dyn FnMut()) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let available = ui.available_width() - 110.0;
                let response = ui.add_sized(
                    vec2(available, 38.0),
                    egui::TextEdit::singleline(input_text)
                        .hint_text("🔗 Collez un lien Spotify ou YouTube…")
                        .font(egui::TextStyle::Body)
                        .margin(Vec2::new(12.0, 10.0)),
                );

                let enter_pressed = response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                ui.add_space(8.0);

                let btn = ui.add_sized(
                    vec2(100.0, 38.0),
                    egui::Button::new(
                        RichText::new("⬇ Télécharger").size(14.0).color(BG_DARK).strong(),
                    )
                    .fill(ACCENT)
                    .corner_radius(CornerRadius::same(8)),
                );

                // Hover effect
                if btn.hovered() {
                    ui.ctx().output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                }

                if (btn.clicked() || enter_pressed) && !input_text.trim().is_empty() {
                    on_submit();
                }
            });
        });
}

// ---------------------------------------------------------------------------
// Settings panel
// ---------------------------------------------------------------------------

fn draw_settings(ui: &mut egui::Ui, config: &mut AppConfig, on_browse_folder: &mut dyn FnMut()) {
    egui::Frame::new()
        .fill(BG_PANEL)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(16))
        .outer_margin(egui::Margin::symmetric(20, 0))
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.label(RichText::new("Paramètres").size(15.0).color(ACCENT).strong());
            ui.add_space(10.0);

            // Output folder
            ui.horizontal(|ui| {
                ui.label(RichText::new("Dossier :").size(13.0).color(TEXT_SECONDARY));
                ui.add_space(8.0);
                let dir_str = config.output_dir.to_string_lossy().to_string();
                ui.label(
                    RichText::new(if dir_str.len() > 50 {
                        format!("…{}", &dir_str[dir_str.len() - 48..])
                    } else {
                        dir_str
                    })
                    .size(13.0)
                    .color(TEXT_PRIMARY),
                );
                ui.add_space(8.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("Parcourir…").size(12.0))
                            .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    on_browse_folder();
                }
            });

            ui.add_space(8.0);

            // Track number toggle
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Numéro de piste dans le nom :")
                        .size(13.0)
                        .color(TEXT_SECONDARY),
                );
                ui.add_space(8.0);
                ui.add(toggle_switch(&mut config.add_track_number));
            });

        });
}

// ---------------------------------------------------------------------------
// Queue header
// ---------------------------------------------------------------------------

fn draw_queue_header(
    ui: &mut egui::Ui,
    queue: &[QueueItem],
    on_open_folder: &mut dyn FnMut(),
    on_clear_done: &mut dyn FnMut(),
) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let done_count = queue
                    .iter()
                    .filter(|q| q.status == DownloadStatus::Done)
                    .count();
                let total = queue.len();

                ui.label(
                    RichText::new(format!("File d'attente ({total})"))
                        .size(14.0)
                        .color(TEXT_PRIMARY)
                        .strong(),
                );

                if done_count > 0 {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("{done_count} terminé(s)"))
                            .size(12.0)
                            .color(Color32::from_rgb(80, 220, 120)),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !queue.is_empty() {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("🗑 Effacer terminés")
                                        .size(12.0)
                                        .color(TEXT_SECONDARY),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            on_clear_done();
                        }
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("📂 Ouvrir dossier")
                                    .size(12.0)
                                    .color(TEXT_SECONDARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        on_open_folder();
                    }
                });
            });
        });
}

// ---------------------------------------------------------------------------
// Queue list
// ---------------------------------------------------------------------------

fn draw_queue(ui: &mut egui::Ui, queue: &[QueueItem]) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 0))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if queue.is_empty() {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Aucun téléchargement en cours")
                                    .size(14.0)
                                    .color(TEXT_SECONDARY),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Collez un lien Spotify ou YouTube ci-dessus")
                                    .size(12.0)
                                    .color(Color32::from_rgb(100, 100, 115)),
                            );
                        });
                    }

                    for item in queue {
                        draw_queue_item(ui, item);
                        ui.add_space(4.0);
                    }
                });
        });
}

fn draw_queue_item(ui: &mut egui::Ui, item: &QueueItem) {
    let status_color = item.status_color();

    egui::Frame::new()
        .fill(BG_CARD)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(12))
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Status indicator dot
                let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(rect.center(), 4.0, status_color);

                ui.add_space(8.0);

                // Track info
                ui.vertical(|ui| {
                    let title_display = if item.title.is_empty() {
                        "Sans titre".to_string()
                    } else {
                        item.title.clone()
                    };

                    ui.label(
                        RichText::new(&title_display)
                            .size(13.5)
                            .color(TEXT_PRIMARY)
                            .strong(),
                    );

                    if !item.artist.is_empty() {
                        ui.label(
                            RichText::new(&item.artist)
                                .size(12.0)
                                .color(TEXT_SECONDARY),
                        );
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Spinner for active states
                    let is_active = matches!(
                        item.status,
                        DownloadStatus::Searching
                            | DownloadStatus::Downloading
                            | DownloadStatus::Converting
                            | DownloadStatus::Tagging
                    );

                    if is_active {
                        ui.spinner();
                        ui.add_space(6.0);
                    }

                    ui.label(
                        RichText::new(item.status.to_string())
                            .size(12.0)
                            .color(status_color),
                    );
                });
            });

            // Progress bar for active downloads
            if matches!(
                item.status,
                DownloadStatus::Searching
                    | DownloadStatus::Downloading
                    | DownloadStatus::Converting
                    | DownloadStatus::Tagging
            ) {
                ui.add_space(6.0);
                let progress = match item.status {
                    DownloadStatus::Searching => 0.15,
                    DownloadStatus::Downloading => 0.45,
                    DownloadStatus::Converting => 0.75,
                    DownloadStatus::Tagging => 0.9,
                    _ => 0.0,
                };
                let bar = egui::ProgressBar::new(progress)
                    .fill(status_color.linear_multiply(0.7))
                    .animate(true);
                ui.add(bar);

                // Request repaint for animation
                ui.ctx().request_repaint();
            }
        });
}

// ---------------------------------------------------------------------------
// Custom toggle switch widget
// ---------------------------------------------------------------------------

fn toggle_switch(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| {
        let desired_size = vec2(36.0, 20.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if response.clicked() {
            *on = !*on;
        }

        let anim_t = ui.ctx().animate_bool_with_time(response.id, *on, 0.15);

        let bg = Color32::from_rgb(
            (60.0 + 195.0 * anim_t) as u8,
            (60.0 + 80.0 * anim_t) as u8,
            (75.0 - 25.0 * anim_t) as u8,
        );

        ui.painter()
            .rect_filled(rect, CornerRadius::same(10), bg);

        let circle_x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), anim_t);
        ui.painter().circle_filled(
            egui::pos2(circle_x, rect.center().y),
            7.0,
            Color32::WHITE,
        );

        response
    }
}
