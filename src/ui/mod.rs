use crate::worker::{Channel, Item, ToApp, ToWorker, Worker, WorkerError};
use crossbeam_channel::{Receiver, Sender};
use eframe::CreationContext;
use egui::{
    Align, Button, CentralPanel, CollapsingHeader, ComboBox, Context, Direction, Frame, Label,
    Layout, Margin, RichText, ScrollArea, TextEdit, TopBottomPanel,
};
use tracing::error;

mod widgets;

#[derive(Default, PartialEq)]
enum Page {
    #[default]
    Feed,
    Channels,
    Settings,
}

#[derive(Default, PartialEq)]
enum FeedTypeCombo {
    #[default]
    New,
    Dismissed,
}

#[derive(Default)]
pub struct TinyrssApp {
    page: Page,
    feed_page: usize,
    channel_input: String,
    feed_type_combo: FeedTypeCombo,

    channels: Vec<Channel>,
    feed_items: Vec<Item>,

    worker_status: WorkerStatus,
    sender: Option<Sender<ToWorker>>,
    receiver: Option<Receiver<ToApp>>,
}

#[derive(Default)]
struct WorkerStatus {
    updating_feed: bool,
    worker_errors: Vec<WorkerError>,
}

impl TinyrssApp {
    pub fn new(cc: &CreationContext) -> Self {
        let mut app = Self::default();

        let (app_tx, app_rx) = crossbeam_channel::unbounded();
        let (worker_tx, worker_rx) = crossbeam_channel::unbounded();

        let context = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            Worker::new(worker_tx, app_rx, context).init();
        });

        app.sender = Some(app_tx);
        app.receiver = Some(worker_rx);

        app.worker_status.updating_feed = true;

        if let Some(sender) = &app.sender {
            sender.send(ToWorker::Startup).unwrap();
        }

        app
    }
}

impl eframe::App for TinyrssApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if let Some(receiver) = &self.receiver {
            if let Ok(message) = receiver.try_recv() {
                match message {
                    ToApp::UpdateFeed { items } => {
                        self.worker_status.updating_feed = false;
                        self.feed_items = items;
                    }
                    ToApp::WorkerError { error } => {
                        error!(
                            "Received error from worker: {} {}",
                            error.description, error.error_message
                        );
                        self.worker_status.worker_errors.push(error);
                    }
                    ToApp::UpdateChannels { channels } => {
                        self.channels = channels;
                    }
                }
            }
        }

        self.render_header(ctx);

        self.render_central_panel(ctx);

        self.render_footer(ctx);
    }
}

impl TinyrssApp {
    fn render_header(&mut self, ctx: &Context) {
        TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.page, Page::Feed, "Feed");
                ui.selectable_value(&mut self.page, Page::Channels, "Channels");
                ui.selectable_value(&mut self.page, Page::Settings, "Settings");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.page == Page::Feed {
                        if ui
                            .add_enabled(!self.worker_status.updating_feed, Button::new("⟳"))
                            .clicked()
                        {
                            self.update_feed();
                        };
                        ComboBox::from_id_source("feed_type_combo")
                            .selected_text(match self.feed_type_combo {
                                FeedTypeCombo::New => "New",
                                FeedTypeCombo::Dismissed => "Dismissed",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.feed_type_combo,
                                    FeedTypeCombo::New,
                                    "New",
                                );
                                ui.selectable_value(
                                    &mut self.feed_type_combo,
                                    FeedTypeCombo::Dismissed,
                                    "Dismissed",
                                );
                            });
                    }
                });
            });
        });
    }

    fn render_central_panel(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Feed => {
                self.render_feed_page(ui);
            }
            Page::Channels => {
                self.render_channels_page(ui);
            }
            Page::Settings => {
                self.render_settings_page(ui);
            }
        });
    }

    fn render_feed_page(&mut self, ui: &mut egui::Ui) {
        if self.worker_status.updating_feed {
            ui.with_layout(
                Layout::centered_and_justified(Direction::LeftToRight),
                |ui| {
                    ui.spinner();
                },
            );
        } else {
            if self.feed_items.is_empty() {
                ui.with_layout(
                    Layout::centered_and_justified(Direction::LeftToRight),
                    |ui| {
                        ui.label("No items in feed");
                    },
                );
                return;
            }

            const ITEMS_PER_PAGE: usize = 15;

            let from = self.feed_page * ITEMS_PER_PAGE;
            let to;
            let last_page: bool;

            let filtered_items: Vec<&Item>;

            match self.feed_type_combo {
                FeedTypeCombo::New => {
                    filtered_items = self
                        .feed_items
                        .iter()
                        .filter(|item| !item.dismissed)
                        .collect();
                }
                FeedTypeCombo::Dismissed => {
                    filtered_items = self
                        .feed_items
                        .iter()
                        .filter(|item| item.dismissed)
                        .collect();
                }
            }

            last_page =
                (filtered_items.len() - (self.feed_page * ITEMS_PER_PAGE)) <= ITEMS_PER_PAGE;

            if from + ITEMS_PER_PAGE > filtered_items.len() {
                to = filtered_items.len();
            } else {
                to = from + ITEMS_PER_PAGE;
            }

            if filtered_items.is_empty() {
                let text;
                match self.feed_type_combo {
                    FeedTypeCombo::New => text = "No new items",
                    FeedTypeCombo::Dismissed => text = "No dismissed items",
                }
                ui.with_layout(
                    Layout::centered_and_justified(Direction::LeftToRight),
                    |ui| {
                        ui.label(text);
                    },
                );
                return;
            } else {
                ScrollArea::vertical().show(ui, |ui| {
                    for item in &filtered_items[from..to] {
                        widgets::feed_card(ui, self.sender.clone(), item);
                    }
                });
            }

            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.feed_page > 0, Button::new("<"))
                        .clicked()
                    {
                        self.feed_page -= 1;
                    }
                    ui.label((self.feed_page + 1).to_string());
                    if ui.add_enabled(!last_page, Button::new(">")).clicked() {
                        self.feed_page += 1;
                    }
                });
                if self.feed_type_combo == FeedTypeCombo::New {
                    ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                        if ui.link("Dismiss all").clicked() {
                            self.dismiss_all();
                        }
                    });
                }
            });
        }
    }

    fn render_channels_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Paste").clicked() {
                let mut ctx = match copypasta::ClipboardContext::new() {
                    Ok(ctx) => ctx,
                    Err(err) => {
                        self.worker_status
                            .worker_errors
                            .push(WorkerError::new("Clipboard error", err.to_string()));
                        return;
                    }
                };
                let clipboard_content = match ctx.get_contents() {
                    Ok(ctx) => ctx,
                    Err(err) => {
                        self.worker_status.worker_errors.push(WorkerError::new(
                            "Failed to access clipboard",
                            err.to_string(),
                        ));
                        return;
                    }
                };
                self.channel_input = clipboard_content;
            }
            if ui
                .add_enabled(!self.channel_input.is_empty(), Button::new("Add"))
                .clicked()
            {
                self.add_channel(&self.channel_input.clone());
                self.channel_input = "".to_string();
            };
            ui.add_sized(
                ui.available_size(),
                TextEdit::singleline(&mut self.channel_input).hint_text("Search or add channels"),
            );
        });

        if self.channels.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("You are not subscribed to any channels");
            });
        } else {
            let search_result_exists = self.channels.iter().any(|channel| {
                if let Some(title) = &channel.title {
                    return title
                        .to_lowercase()
                        .contains(self.channel_input.to_lowercase().as_str());
                }
                return false;
            });

            if !search_result_exists && !self.channels.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No channels matched your search");
                });
            } else {
                ScrollArea::vertical().show(ui, |ui| {
                    for channel in &self.channels {
                        widgets::channel_card(
                            ui,
                            self.sender.clone(),
                            channel,
                            &self.channel_input,
                        );
                    }
                });
            }
        }
    }

    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new(RichText::new("Channels").strong().heading())
            .default_open(false)
            .show(ui, |ui| {
                if ui.button("Import").clicked() {
                    if let Some(sender) = &self.sender {
                        sender.send(ToWorker::ImportChannels).unwrap();
                    }
                }
                if ui.button("Export").clicked() {
                    if let Some(sender) = &self.sender {
                        sender.send(ToWorker::ExportChannels).unwrap();
                    }
                }
            });
    }

    fn render_footer(&mut self, ctx: &Context) {
        if self.worker_status.worker_errors.len() > 0 {
            TopBottomPanel::bottom("footer").show(ctx, |ui| {
                self.worker_status.worker_errors.retain(|error| {
                    let mut retain = true;

                    Frame {
                        fill: egui::Color32::from_rgb(200, 0, 0),
                        inner_margin: Margin::same(4.0),
                        rounding: egui::Rounding::same(4.0),
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                Label::new(format!(
                                    "{}: {}",
                                    error.description, error.error_message
                                ))
                                .wrap(true),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Close").clicked() {
                                    retain = false;
                                }
                            });
                        });
                    });

                    retain
                });
            });
        }
    }
}

impl TinyrssApp {
    fn add_channel(&mut self, link: &str) {
        if let Some(sender) = &self.sender {
            sender
                .send(ToWorker::AddChannel { link: link.into() })
                .unwrap();
        }
    }

    fn update_feed(&mut self) {
        self.worker_status.updating_feed = true;
        if let Some(sender) = &self.sender {
            sender.send(ToWorker::UpdateFeed).unwrap();
        }
    }

    fn dismiss_all(&mut self) {
        if let Some(sender) = &self.sender {
            sender.send(ToWorker::DismissAll).unwrap();
        }
    }
}
