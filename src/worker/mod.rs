use crossbeam_channel::{Receiver, Sender};
pub use messages::{ToApp, ToWorker, WorkerError};
use parking_lot::Once;
use tracing::{error, info};

mod db;
mod messages;
mod utils;

static CHANNEL_CLOSED: Once = Once::new();

pub struct Worker {
    sender: Sender<ToApp>,
    receiver: Receiver<ToWorker>,
    egui_ctx: eframe::egui::Context,
}

impl Worker {
    pub fn new(
        sender: Sender<ToApp>,
        receiver: Receiver<ToWorker>,
        egui_ctx: eframe::egui::Context,
    ) -> Self {
        Self {
            sender,
            receiver,
            egui_ctx,
        }
    }

    pub fn init(&mut self) {
        info!("Worker starting up.");

        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            loop {
                match self.receiver.recv() {
                    Ok(message) => {
                        match message {
                            ToWorker::Startup => {
                                self.initialize_app_fs();

                                self.initialize_database().await;
                            }
                            ToWorker::UpdateFeed => {
                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                self.sender
                                    .send(ToApp::UpdateFeed { entries: vec![] })
                                    .unwrap();
                            }
                        }
                        self.egui_ctx.request_repaint();
                    }
                    Err(err) => {
                        CHANNEL_CLOSED.call_once(|| {
                            error!("Failed to process message from app: {}", err);
                        });
                    }
                }
            }
        });
    }

    fn initialize_app_fs(&mut self) {
        let app_dir = utils::get_app_dir();
        let db_path = app_dir.join("tinyrss.db");

        if let Err(err) = std::fs::create_dir_all(utils::get_app_dir()) {
            self.report_error("Failed to initialize app filesystem", err.to_string());
        } else {
            info!("Initialized application filesystem.");
        };

        if !db_path.exists() {
            if let Err(err) = std::fs::File::create(db_path) {
                self.report_error("Failed to create database", err.to_string());
            };
        }
    }

    async fn initialize_database(&mut self) {
        if let Err(err) = db::create_tables().await {
            self.report_error("Failed to initialize database", err.to_string());
        } else {
            info!("Initialized database.");
        };
    }

    fn report_error(&mut self, description: impl Into<String>, message: impl Into<String>) {
        self.sender
            .send(ToApp::WorkerError {
                error: WorkerError::new(description, message),
            })
            .unwrap();
    }
}
