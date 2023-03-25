use crate::worker::{ToApp, ToWorker, Worker, WorkerError};
use crossbeam_channel::{Receiver, Sender};
use eframe::CreationContext;
use egui::{Align, CentralPanel, Context, Frame, Label, Layout, Margin, TopBottomPanel};
use tracing::error;

#[derive(Default, PartialEq)]
enum Page {
    #[default]
    Feed,
    Channels,
    Settings,
}

#[derive(Default)]
pub struct TinyrssApp {
    page: Page,

    channels: Vec<i32>,
    feed_entries: Vec<i32>,

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
                    ToApp::UpdateFeed { entries } => {
                        self.worker_status.updating_feed = false;
                        self.feed_entries = entries;
                        self.worker_status.worker_errors.push(WorkerError {
                            description: "Test".into(),
                            error_message: "Test".into(),
                        })
                    }
                    ToApp::WorkerError { error } => {
                        error!(
                            "Received error from worker: {} {}",
                            error.description, error.error_message
                        );
                        self.worker_status.worker_errors.push(error);
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
            })
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
            ui.spinner();
        } else {
            ui.heading("Feed page");
            if ui.button("Update feed").clicked() {
                if let Some(sender) = &self.sender {
                    self.worker_status.updating_feed = true;
                    sender.send(ToWorker::UpdateFeed).unwrap();
                }
            }
        }
    }

    fn render_channels_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Channels page");
    }

    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings page");
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
