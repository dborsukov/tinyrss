use crate::worker::{Channel, ConfigBuilder, Item, ToApp, ToWorker, Worker, WorkerError, CONFIG};
use copypasta::ClipboardProvider;
use crossbeam_channel::{Receiver, Sender};
use eframe::CreationContext;
use egui::{
    Align, Button, CentralPanel, CollapsingHeader, ComboBox, Context, Direction, Frame, Label,
    Layout, Margin, ProgressBar, RichText, ScrollArea, TextEdit, TopBottomPanel, Vec2,
};
use lazy_static::lazy_static;
use theme::{Colors, Theme};
use tracing::error;

mod theme;
mod widgets;

lazy_static! {
    static ref THEME: Theme = Theme::from_colors(Colors::dark());
}

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
    feed_input: String,
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
    update_progress: f32,
    importing_channels: bool,
    import_progress: f32,
    worker_errors: Vec<WorkerError>,
}

impl TinyrssApp {
    pub fn new(cc: &CreationContext) -> Self {
        let mut app = Self::default();

        app.configure_styles(&cc.egui_ctx);

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
                        self.worker_status.update_progress = 0.0;
                        self.feed_items = items;
                    }
                    ToApp::FeedUpdateProgress { progress } => {
                        self.worker_status.update_progress = progress;
                    }
                    ToApp::WorkerError { error } => {
                        error!(
                            "Received error from worker: {} {}",
                            error.description, error.error_message
                        );
                        self.worker_status.worker_errors.push(error);
                    }
                    ToApp::UpdateChannels { channels } => {
                        self.worker_status.importing_channels = false;
                        self.worker_status.import_progress = 0.0;
                        self.channels = channels;
                    }
                    ToApp::ImportProgress { progress } => {
                        self.worker_status.import_progress = progress;
                    }
                }
            }
        }

        self.render_header(ctx);

        self.render_central_panel(ctx);

        self.render_footer(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(sender) = &self.sender {
            sender.send(ToWorker::Shutdown).unwrap();
        }
    }
}

impl TinyrssApp {
    fn render_header(&mut self, ctx: &Context) {
        TopBottomPanel::top("header")
            .min_height(30.)
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
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
                                    if ui
                                        .selectable_value(
                                            &mut self.feed_type_combo,
                                            FeedTypeCombo::New,
                                            "New",
                                        )
                                        .changed()
                                    {
                                        self.feed_page = 0;
                                    };
                                    if ui
                                        .selectable_value(
                                            &mut self.feed_type_combo,
                                            FeedTypeCombo::Dismissed,
                                            "Dismissed",
                                        )
                                        .changed()
                                    {
                                        self.feed_page = 0;
                                    };
                                });
                            if CONFIG.lock().show_search_in_feed {
                                if ui
                                    .add(
                                        TextEdit::singleline(&mut self.feed_input)
                                            .hint_text("Search"),
                                    )
                                    .changed()
                                {
                                    self.feed_page = 0;
                                };
                            }
                        }
                    });
                });
            });
    }

    fn render_central_panel(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Feed => {
                self.render_feed_page(ctx, ui);
            }
            Page::Channels => {
                self.render_channels_page(ui);
            }
            Page::Settings => {
                self.render_settings_page(ctx, ui);
            }
        });
    }

    fn render_feed_page(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        if self.worker_status.updating_feed {
            ui.with_layout(
                Layout::centered_and_justified(Direction::LeftToRight),
                |ui| {
                    ui.add(
                        ProgressBar::new(self.worker_status.update_progress)
                            .desired_width(300.0)
                            .animate(true),
                    )
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

            const ITEMS_PER_PAGE: usize = 10;

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
                        .filter(|item| {
                            item.title
                                .clone()
                                .unwrap()
                                .to_lowercase()
                                .contains(self.feed_input.to_lowercase().as_str())
                        })
                        .collect();
                }
                FeedTypeCombo::Dismissed => {
                    filtered_items = self
                        .feed_items
                        .iter()
                        .filter(|item| item.dismissed)
                        .filter(|item| {
                            item.title
                                .clone()
                                .unwrap()
                                .to_lowercase()
                                .contains(self.feed_input.to_lowercase().as_str())
                        })
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
                        ui.add_space(THEME.spacing.medium);
                    }
                });
            }

            ui.horizontal_centered(|ui| {
                ui.spacing_mut().button_padding = Vec2::new(10., 2.);
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
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
                });

                let modal = egui_modal::Modal::new(ctx, "modal_dismiss_all");

                modal.show(|ui| {
                    modal.title(ui, "Warning");
                    let amount = self
                        .feed_items
                        .iter()
                        .filter(|item| !item.dismissed)
                        .count();
                    modal.body(ui, format!("All new items will be dismissed! ({})", amount));
                    modal.buttons(ui, |ui| {
                        ui.spacing_mut().button_padding = Vec2::new(8., 4.);
                        if ui.add(Button::new("Close")).clicked() {
                            modal.close();
                        };
                        if ui
                            .add(Button::new("Confirm").fill(THEME.colors.warning))
                            .clicked()
                        {
                            self.dismiss_all();
                            modal.close();
                        };
                    });
                });

                ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
                    if self.feed_type_combo == FeedTypeCombo::New {
                        ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                            if ui.link("Dismiss all").clicked() {
                                modal.open();
                            }
                        });
                    }
                });
            });
        }
    }

    fn render_channels_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(6., 4.);
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
                TextEdit::singleline(&mut self.channel_input)
                    .hint_text("Search or add channels")
                    .margin(Vec2::new(6., 3.)),
            );
        });

        if self.channels.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("You are not subscribed to any channels");
            });
        } else {
            ui.add_space(THEME.spacing.medium);
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

    fn render_settings_page(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        if self.worker_status.importing_channels {
            ui.with_layout(
                Layout::centered_and_justified(Direction::LeftToRight),
                |ui| {
                    ui.add(
                        ProgressBar::new(self.worker_status.import_progress)
                            .desired_width(300.0)
                            .text("Import in progress...")
                            .animate(true),
                    )
                },
            );
        } else {
            ScrollArea::vertical().show(ui, |ui| {
                self.render_general_settings(ctx, ui);
                ui.add_space(THEME.spacing.large);
                self.render_channels_settings(ctx, ui);
            });
        }
    }

    fn render_general_settings(&mut self, _ctx: &Context, ui: &mut egui::Ui) {
        CollapsingHeader::new(RichText::new("General").strong().heading())
            .default_open(true)
            .show(ui, |ui| {
                ui.add_space(THEME.spacing.large);
                ui.horizontal(|ui| {
                    ui.label("Auto dismiss");
                    ui.label(RichText::new("(?)").color(THEME.colors.text_dim).monospace()).on_hover_text("Dismiss items just by opening them.");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .checkbox(&mut CONFIG.lock().auto_dismiss_on_open, "")
                            .changed()
                        {
                            ConfigBuilder::from_current().apply();
                        };
                    });
                });
                ui.add_space(THEME.spacing.large);
                ui.horizontal(|ui| {
                    ui.label("Show feed search");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .checkbox(&mut CONFIG.lock().show_search_in_feed, "")
                            .changed()
                        {
                            self.feed_input = String::new();
                            ConfigBuilder::from_current().apply();
                        };
                    });
                });
                ui.add_space(THEME.spacing.large);
                ui.horizontal(|ui| {
                    ui.label("Concurent requests");
                    ui.label(RichText::new("(?)").color(THEME.colors.text_dim).monospace()).on_hover_text("Amount of network requests that will happen at the same time.\nHigher amount may lead to faster load times.");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(egui::Slider::new(
                                &mut CONFIG.lock().max_allowed_concurent_requests,
                                1..=10,
                            ))
                            .changed()
                        {
                            ConfigBuilder::from_current().apply();
                        };
                    });
                });
            });
    }

    fn render_channels_settings(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        CollapsingHeader::new(RichText::new("Channels").strong().heading())
            .default_open(true)
            .show(ui, |ui| {
                ui.spacing_mut().button_padding = Vec2::new(6., 3.);
                ui.add_space(THEME.spacing.large);
                ui.horizontal(|ui| {
                    ui.label("OPML");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Import").clicked() {
                            if let Some(sender) = &self.sender {
                                let path = rfd::FileDialog::new()
                                    .add_filter("OPML", &["xml", "opml"])
                                    .pick_file();
                                self.worker_status.importing_channels = true;
                                sender.send(ToWorker::ImportChannels { path }).unwrap();
                            }
                        }
                        if ui.button("Export").clicked() {
                            if let Some(sender) = &self.sender {
                                sender.send(ToWorker::ExportChannels).unwrap();
                            }
                        }
                    })
                });
                ui.add_space(THEME.spacing.large);

                let modal = egui_modal::Modal::new(ctx, "modal_manage_channels");

                if !self.channels.is_empty() {
                    let combo_id = ui.id().with("combo_channel");
                    let mut combo_channel = ui.data_mut(|d| {
                        d.get_temp::<String>(combo_id)
                            .unwrap_or(self.channels[0].id.clone())
                    });

                    let edit_title_id = ui.id().with("edit_title");
                    let mut edit_title = ui
                        .data_mut(|d| d.get_temp::<String>(edit_title_id).unwrap_or(String::new()));

                    modal.show(|ui| {
                        modal.title(ui, "Manage channels");
                        modal.frame(ui, |ui| {
                            ui.add_space(THEME.spacing.medium);
                            ui.horizontal(|ui| {
                                ui.label("Channel:");
                                ComboBox::from_id_source("channel_choose_combo")
                                    .selected_text(
                                        self.channels
                                            .iter()
                                            .find(|c| c.id == combo_channel)
                                            .unwrap()
                                            .title
                                            .clone()
                                            .unwrap_or("<no title>".to_string()),
                                    )
                                    .wrap(true)
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for channel in &self.channels {
                                            ui.selectable_value(
                                                &mut combo_channel,
                                                channel.id.clone(),
                                                channel
                                                    .title
                                                    .clone()
                                                    .unwrap_or("<no title>".to_string()),
                                            );
                                        }
                                    });
                            });
                            ui.add_space(THEME.spacing.large);
                            ui.horizontal(|ui| {
                                ui.label("New title:");
                                ui.add(
                                    TextEdit::singleline(&mut edit_title)
                                        .desired_width(ui.available_width()),
                                );
                            });
                        });
                        modal.buttons(ui, |ui| {
                            ui.spacing_mut().button_padding = Vec2::new(8., 4.);
                            if ui.add(Button::new("Close")).clicked() {
                                modal.close();
                            };
                            if ui
                                .add_enabled(!edit_title.is_empty(), Button::new("Save"))
                                .clicked()
                            {
                                let channel = self
                                    .channels
                                    .iter()
                                    .find(|c| c.id == combo_channel)
                                    .unwrap();
                                if let Some(sender) = &self.sender {
                                    sender
                                        .send(ToWorker::EditChannel {
                                            id: channel.id.clone(),
                                            title: edit_title.clone(),
                                        })
                                        .unwrap();
                                }
                                modal.close();
                            };
                        });
                    });

                    ui.data_mut(|d| d.insert_temp(combo_id, combo_channel));
                    ui.data_mut(|d| d.insert_temp(edit_title_id, edit_title));
                }

                ui.horizontal(|ui| {
                    ui.label("Manage channels");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(!self.channels.is_empty(), Button::new("Manage"))
                            .clicked()
                        {
                            modal.open();
                        }
                    })
                });
            });
    }

    fn render_footer(&mut self, ctx: &Context) {
        if self.worker_status.worker_errors.len() > 0 {
            TopBottomPanel::bottom("footer")
                .frame(Frame {
                    fill: THEME.colors.bg_darker,
                    inner_margin: Margin::same(6.0),
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    self.worker_status.worker_errors.retain(|error| {
                        let mut retain = true;

                        Frame {
                            fill: THEME.colors.warning,
                            inner_margin: Margin::same(6.0),
                            rounding: THEME.rounding.medium,
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

impl TinyrssApp {
    fn configure_styles(&mut self, ctx: &egui::Context) {
        use egui::style::{DebugOptions, TextStyle};
        use egui::FontFamily::{Monospace, Proportional};
        use egui::{FontId, Style};

        let style = Style {
            visuals: THEME.visuals.clone(),
            text_styles: [
                (TextStyle::Small, FontId::new(8.0, Proportional)),
                (TextStyle::Body, FontId::new(16.0, Proportional)),
                (TextStyle::Monospace, FontId::new(12.0, Monospace)),
                (TextStyle::Button, FontId::new(14.0, Proportional)),
                (TextStyle::Heading, FontId::new(22.0, Proportional)),
            ]
            .into(),
            debug: DebugOptions {
                debug_on_hover: false,
                show_expand_width: false,
                show_expand_height: false,
                show_resize: false,
                show_interactive_widgets: false,
                show_blocking_widget: false,
            },
            ..Style::default()
        };

        ctx.set_style(style);
    }
}
