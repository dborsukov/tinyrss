use crate::worker::db;

pub enum ToApp {
    WorkerError { error: WorkerError },
    UpdateFeed { items: Vec<db::Item> },
    UpdateChannels { channels: Vec<db::Channel> },
}

pub enum ToWorker {
    Startup,
    UpdateFeed,
    AddChannel { link: String },
    SetDismissed { id: String, dismissed: bool },
    DismissAll,
    Unsubscribe { id: String },
    ImportChannels,
    ExportChannels
}

pub struct WorkerError {
    pub description: String,
    pub error_message: String,
}

impl WorkerError {
    pub fn new(description: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            error_message: error_message.into(),
        }
    }
}
