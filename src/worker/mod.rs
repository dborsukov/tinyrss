use crossbeam_channel::{Receiver, Sender};
pub use db::{Channel, Item};
pub use messages::{ToApp, ToWorker, WorkerError};
use parking_lot::Once;
use tracing::{error, info};

mod db;
mod messages;
mod utils;

static CHANNEL_CLOSED: Once = Once::new();

struct FeedParsingError;

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

                                // self.parse_channels().await;

                                self.update_feed().await;
                            }
                            ToWorker::UpdateFeed => {
                                self.parse_channels().await;

                                self.update_feed().await;
                            }
                            ToWorker::AddChannel { link } => {
                                self.add_channel(&link).await;

                                self.update_channel_list().await;
                            }
                            ToWorker::SetDismissed { id, dismissed } => {
                                self.set_dismissed(&id, dismissed).await;

                                self.update_feed().await;
                            }
                            ToWorker::DismissAll => {
                                self.dismiss_all().await;

                                self.update_feed().await;
                            }
                            ToWorker::Unsubscribe { id } => {
                                self.unsubscribe(&id).await;

                                self.update_channel_list().await;

                                self.update_feed().await;
                            }
                            ToWorker::ImportChannels => {
                                self.import_channels().await;

                                self.update_channel_list().await;
                            }
                            ToWorker::ExportChannels => {
                                self.export_channels().await;
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

    async fn parse_xml_link(
        &mut self,
        link: &str,
    ) -> Result<feed_rs::model::Feed, FeedParsingError> {
        let result = match reqwest::get(link).await {
            Ok(result) => result,
            Err(err) => {
                self.report_error("Web request failed", err.to_string());
                return Err(FeedParsingError);
            }
        };
        let bytes = match result.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                self.report_error("Malformed response", err.to_string());
                return Err(FeedParsingError);
            }
        };
        let parsed_feed = match feed_rs::parser::parse(&bytes[..]) {
            Ok(feed) => feed,
            Err(err) => {
                self.report_error("Failed to parse response", err.to_string());
                return Err(FeedParsingError);
            }
        };
        Ok(parsed_feed)
    }

    async fn add_channel(&mut self, link: &str) {
        let parsed_feed = match self.parse_xml_link(link).await {
            Ok(feed) => feed,
            Err(_) => return,
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

    async fn parse_channels(&mut self) {
        let mut items: Vec<Item> = vec![];

        let channels = match db::get_all_channels().await {
            Ok(channels) => channels,
            Err(err) => {
                self.report_error("Failed to fetch channel from db", err.to_string());
                return;
            }
        };

        for channel in channels {
            let parsed_entries = match self.parse_xml_link(&channel.link).await {
                Ok(feed) => feed.entries,
                Err(_) => return,
            };
            for entry in parsed_entries {
                let mut item = Item {
                    id: entry.id,
                    channel_title: channel.title.clone(),
                    channel: channel.id.clone(),
                    dismissed: false,
                    ..Default::default()
                };

                if entry.links.len() > 0 {
                    item.link = entry.links[0].href.clone();
                } else {
                    item.link = "<no link>".to_string();
                }

                item.title = match entry.title {
                    Some(text) => Some(text.content),
                    None => None,
                };

                item.summary = match entry.summary {
                    Some(text) => Some(text.content),
                    None => None,
                };

                if entry.published.is_some() {
                    item.published = entry.published.unwrap().timestamp()
                } else if entry.updated.is_some() {
                    item.published = entry.updated.unwrap().timestamp()
                } else {
                    item.published = 0;
                }

                items.push(item);
            }
        }

        if let Err(err) = db::add_items(items).await {
            self.report_error("Failed to save new feed items", err.to_string())
        };

        info!("Feed update finished.");
    }

    async fn update_feed(&mut self) {
        let items = match db::get_all_items().await {
            Ok(items) => items,
            Err(err) => {
                self.report_error("Failed to fetch items from db", err.to_string());
                return;
            }
        };

        self.sender.send(ToApp::UpdateFeed { items }).unwrap();
    }

    async fn set_dismissed(&mut self, id: &str, dismissed: bool) {
        if let Err(err) = db::set_dismissed(id, dismissed).await {
            self.report_error("Falied to set dismissed", err.to_string());
        }
    }

    async fn dismiss_all(&mut self) {
        if let Err(err) = db::dismiss_all().await {
            self.report_error("Falied to dismiss all", err.to_string());
        }
    }

    async fn unsubscribe(&mut self, id: &str) {
        if let Err(err) = db::unsubscribe(id).await {
            self.report_error("Falied to unsubscribe", err.to_string());
        }
    }

    async fn import_channels(&mut self) {
        let file_handle = rfd::AsyncFileDialog::new()
            .add_filter("OPML", &["xml"])
            .pick_file()
            .await;
        if let Some(file_handle) = file_handle {
            let xml = match std::fs::read_to_string(file_handle.path()) {
                Ok(string) => string,
                Err(err) => {
                    self.report_error("Failed to read file", err.to_string());
                    return;
                }
            };
            let opml = match opml::OPML::from_str(&xml) {
                Ok(opml) => opml,
                Err(err) => {
                    self.report_error("Failed to parse xml", err.to_string());
                    return;
                }
            };
            let mut parsed_channels = 0;
            for outline in opml.body.outlines {
                parsed_channels += self.traverse_outline_and_add_channels(outline).await;
            }
            info!("Import finished. Parsed channels: {}", parsed_channels);
        }
    }

    #[async_recursion::async_recursion]
    async fn traverse_outline_and_add_channels(&mut self, root_outline: opml::Outline) -> i32 {
        let mut parsed_channels = 0;
        for outline in root_outline.outlines {
            parsed_channels += self.traverse_outline_and_add_channels(outline).await;
        }
        if let Some(link) = root_outline.xml_url {
            self.add_channel(&link).await;
            parsed_channels += 1;
        };
        parsed_channels
    }

    async fn export_channels(&mut self) {
        let file_handle = rfd::AsyncFileDialog::new()
            .add_filter("OPML", &["xml"])
            .save_file()
            .await;
        if let Some(file_handle) = file_handle {
            let xml = r#"<opml version="2.0"><head/><body><outline text="Outline"/></body></opml>"#;
            let mut opml = match opml::OPML::from_str(&xml) {
                Ok(opml) => opml,
                Err(err) => {
                    self.report_error("Failed to parse xml", err.to_string());
                    return;
                }
            };
            let channels = match db::get_all_channels().await {
                Ok(channels) => channels,
                Err(err) => {
                    self.report_error("Failed to fetch channel from db", err.to_string());
                    return;
                }
            };

            let mut group = opml::Outline::default();

            for channel in channels {
                group.add_feed(
                    &channel.title.unwrap_or("Unknown".to_string()),
                    &channel.link,
                );
            }

            opml.body.outlines.push(group);

            let mut file = match std::fs::File::create(file_handle.path()) {
                Ok(file) => file,
                Err(err) => {
                    self.report_error("Failed to create file", err.to_string());
                    return;
                }
            };
            if let Err(err) = opml.to_writer(&mut file) {
                self.report_error("Failed to write file", err.to_string());
            };
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
