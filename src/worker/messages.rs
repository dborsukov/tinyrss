pub enum ToApp {
    WorkerError { error: WorkerError },
    UpdateFeed { entries: Vec<i32> },
}

pub enum ToWorker {
    Startup,
    UpdateFeed,
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
