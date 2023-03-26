use crossbeam_channel::{Receiver, Sender};
pub use db::{Channel, Item};
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

                                self.update_channel_list().await;
                            }
                            ToWorker::UpdateFeed => {
                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                self.sender
                                    .send(ToApp::UpdateFeed { entries: vec![] })
                                    .unwrap();
                            }
                            ToWorker::AddChannel { link } => {
                                self.add_channel(&link).await;

                                self.update_channel_list().await;
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

    async fn add_channel(&mut self, link: &str) {
        let result = match reqwest::get(link).await {
            Ok(result) => result,
            Err(err) => {
                self.report_error("Web request failed", err.to_string());
                return;
            }
        };
        let bytes = match result.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                self.report_error("Malformed response", err.to_string());
                return;
            }
        };
        let parsed_feed = match feed_rs::parser::parse(&bytes[..]) {
            Ok(feed) => feed,
            Err(err) => {
                self.report_error("Failed to parse response", err.to_string());
                return;
            }
        };
        let mut channel = db::Channel {
            id: parsed_feed.id,
            ..Default::default()
        };
        channel.kind = match parsed_feed.feed_type {
            feed_rs::model::FeedType::Atom => "Atom".into(),
            feed_rs::model::FeedType::JSON => "JSON".into(),
            feed_rs::model::FeedType::RSS0 => "RSS0".into(),
            feed_rs::model::FeedType::RSS1 => "RSS1".into(),
            feed_rs::model::FeedType::RSS2 => "RSS2".into(),
        };
        channel.link = link.into();
        channel.title = match parsed_feed.title {
            Some(text) => Some(text.content),
            None => None,
        };
        channel.description = match parsed_feed.description {
            Some(text) => Some(text.content),
            None => None,
        };
        if let Err(err) = db::add_channel(channel).await {
            self.report_error("Failed to save new channel", err.to_string())
        };
        info!("Added new channel to database. (link: {})", link);
    }

    async fn update_channel_list(&mut self) {
        let channels = match db::get_all_channels().await {
            Ok(channels) => channels,
            Err(err) => {
                self.report_error("Failed to fetch channel from db", err.to_string());
                return;
            }
        };

        self.sender
            .send(ToApp::UpdateChannels { channels })
            .unwrap();
    }

    fn report_error(&mut self, description: impl Into<String>, message: impl Into<String>) {
        self.sender
            .send(ToApp::WorkerError {
                error: WorkerError::new(description, message),
            })
            .unwrap();
    }
}
