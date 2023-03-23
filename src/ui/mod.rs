use crate::worker::{ToApp, ToWorker, Worker, WorkerError};
use crossbeam_channel::{Receiver, Sender};
use eframe::CreationContext;
use egui::{CentralPanel, Context};

#[derive(PartialEq)]
enum Page {
    Feed,
    Channels,
    Settings,
}

impl Default for Page {
    fn default() -> Self {
        Page::Feed
    }
}

#[derive(Default)]
pub struct TinyrssApp {
    page: Page,
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
                // TODO: handle messages from worker and update app state
            }
        }

        self.render_central_panel(ctx);
    }
}

impl TinyrssApp {
    fn render_central_panel(&mut self, ctx: &Context) -> egui::InnerResponse<()> {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.page, Page::Feed, "Feed");
                ui.selectable_value(&mut self.page, Page::Channels, "Channels");
                ui.selectable_value(&mut self.page, Page::Settings, "Settings");
            });
            ui.separator();
            match self.page {
                Page::Feed => {
                    self.render_feed_page(ui);
                }
                Page::Channels => {
                    self.render_channels_page(ui);
                }
                Page::Settings => {
                    self.render_settings_page(ui);
                }
            }
        })
    }

    fn render_feed_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Feed page");
    }

    fn render_channels_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Channels page");
    }

    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings page");
    }
}
