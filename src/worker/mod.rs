use bytes::Bytes;
pub use config::{ConfigBuilder, CONFIG};
use crossbeam_channel::{Receiver, Sender};
pub use db::{Channel, Item};
use feed_rs::model::Feed;
use futures::{stream, StreamExt};
pub use messages::{ToApp, ToWorker, WorkerError};
use parking_lot::{Mutex, Once};
use reqwest::Client;
use std::sync::Arc;
use tracing::{error, info};

mod config;
mod db;
mod messages;
mod utils;

static CHANNEL_CLOSED: Once = Once::new();

#[derive(Debug)]
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
                            ToWorker::Shutdown => {
                                info!("Saving config.");
                                if let Err(err) = ConfigBuilder::from_current().save() {
                                    error!("Failed to save config: {}", err.to_string());
                                };
                                info!("Shutting down.");
                                std::process::exit(0);
                            }
                            ToWorker::UpdateFeed => {
                                self.parse_channels().await;

                                self.update_feed().await;
                            }
                            ToWorker::AddChannel { link } => {
                                self.add_channels(vec![link]).await;

                                self.update_channel_list().await;
                            }
                            ToWorker::EditChannel { id, title } => {
                                self.edit_channel(id, title).await;

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

    async fn add_channels(&mut self, links: Vec<String>) {
        let client = Client::new();

        struct LinkBytesBinding {
            link: String,
            bytes: Option<Bytes>,
        }

        let results = stream::iter(links)
            .map(|link| {
                let client = &client;
                let sender = self.sender.clone();
                async move {
                    let resp = match client.get(&link).send().await {
                        Ok(r) => r,
                        Err(err) => {
                            sender
                                .send(ToApp::WorkerError {
                                    error: WorkerError::new("Web request failed", err.to_string()),
                                })
                                .unwrap();
                            return LinkBytesBinding { link, bytes: None };
                        }
                    };
                    let res = resp.bytes().await;
                    match res {
                        Ok(bytes) => LinkBytesBinding {
                            link,
                            bytes: Some(bytes),
                        },
                        Err(_) => LinkBytesBinding { link, bytes: None },
                    }
                }
            })
            .buffer_unordered(CONFIG.lock().max_allowed_concurent_requests);

        struct LinkFeedBinding {
            link: String,
            feed: Option<Feed>,
        }

        let mut bindings: Vec<LinkFeedBinding> = vec![];

        bindings = results
            .fold(bindings, |mut bindings, r| async {
                match r.bytes {
                    Some(bytes) => {
                        let feed = if let Ok(feed) = feed_rs::parser::parse(&bytes[..]) {
                            Some(feed)
                        } else {
                            None
                        };
                        bindings.push(LinkFeedBinding { link: r.link, feed })
                    }
                    None => bindings.push(LinkFeedBinding {
                        link: r.link,
                        feed: None,
                    }),
                }
                bindings
            })
            .await;

        let mut channels: Vec<Channel> = vec![];

        for binding in bindings {
            let link = binding.link;
            let parsed_feed = match binding.feed {
                Some(feed) => feed,
                None => continue,
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
            channel.link = link.clone();
            channel.title = match parsed_feed.title {
                Some(text) => Some(text.content),
                None => None,
            };
            channel.description = match parsed_feed.description {
                Some(text) => Some(text.content),
                None => None,
            };
            channels.push(channel);
        }
        info!(
            "Saving new channels to database. (amount: {})",
            channels.len()
        );
        if let Err(err) = db::add_channels(channels).await {
            self.report_error("Failed to save new channels", err.to_string())
        };
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

    async fn edit_channel(&mut self, id: String, title: String) {
        if let Err(err) = db::edit_channel(id, title).await {
            self.report_error("Falied to edit channel", err.to_string());
        }
    }

    async fn parse_channels(&mut self) {
        let channels = match db::get_all_channels().await {
            Ok(channels) => channels,
            Err(err) => {
                self.report_error("Failed to fetch channel from db", err.to_string());
                return;
            }
        };

        let channels_total: f32 = channels.len() as f32;

        info!("Started parsing.");

        let client = Client::new();

        struct ChannelBytesBinding {
            channel: Channel,
            bytes: Option<Bytes>,
        }

        let results = stream::iter(channels)
            .map(|channel| {
                let client = &client;
                let sender = self.sender.clone();
                async move {
                    let resp = match client.get(&channel.link).send().await {
                        Ok(r) => r,
                        Err(err) => {
                            sender
                                .send(ToApp::WorkerError {
                                    error: WorkerError::new("Web request failed", err.to_string()),
                                })
                                .unwrap();
                            return ChannelBytesBinding {
                                channel,
                                bytes: None,
                            };
                        }
                    };
                    let res = resp.bytes().await;
                    match res {
                        Ok(bytes) => ChannelBytesBinding {
                            channel,
                            bytes: Some(bytes),
                        },
                        Err(_) => ChannelBytesBinding {
                            channel,
                            bytes: None,
                        },
                    }
                }
            })
            .buffer_unordered(CONFIG.lock().max_allowed_concurent_requests);

        struct ChannelFeedBinding {
            channel: Channel,
            feed: Option<Feed>,
        }

        let mut bindings: Vec<ChannelFeedBinding> = vec![];

        let processed_channels: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));

        bindings = results
            .fold(bindings, |mut bindings, r| async {
                let sender = self.sender.clone();
                let processed_arc = Arc::clone(&processed_channels);
                let mut processed = processed_arc.lock();
                *processed += 1.0;
                sender
                    .send(ToApp::FeedUpdateProgress {
                        progress: *processed / channels_total,
                    })
                    .unwrap();
                match r.bytes {
                    Some(bytes) => {
                        let feed = if let Ok(feed) = feed_rs::parser::parse(&bytes[..]) {
                            Some(feed)
                        } else {
                            None
                        };
                        bindings.push(ChannelFeedBinding {
                            channel: r.channel,
                            feed,
                        })
                    }
                    None => bindings.push(ChannelFeedBinding {
                        channel: r.channel,
                        feed: None,
                    }),
                }
                bindings
            })
            .await;

        info!("Finished parsing.");

        let mut items: Vec<Item> = vec![];

        for binding in bindings {
            if binding.feed.is_none() {
                continue;
            }
            let channel = binding.channel;
            let feed = binding.feed.unwrap();
            for entry in feed.entries {
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

        info!(
            "Saving retrieved items to database (amount: {})",
            items.len()
        );

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
            let mut links: Vec<String> = vec![];
            for outline in opml.body.outlines {
                links.append(&mut self.traverse_outlines(outline).await);
            }
            info!("Amount of links collected: {}", links.len());
            self.add_channels(links).await;
        }
    }

    #[async_recursion::async_recursion]
    async fn traverse_outlines(&mut self, root_outline: opml::Outline) -> Vec<String> {
        let mut links: Vec<String> = vec![];
        for outline in root_outline.outlines {
            links.append(&mut self.traverse_outlines(outline).await);
        }
        if let Some(link) = root_outline.xml_url {
            links.push(link);
        };
        links
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
