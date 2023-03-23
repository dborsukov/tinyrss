use crossbeam_channel::{Receiver, Sender};
use tracing::info;

pub enum ToApp {}

pub enum ToWorker {
    Startup,
}

pub struct WorkerError {}

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
                                info!("Received startup message!");
                            }
                        }
                        self.egui_ctx.request_repaint();
                    }
                    Err(_) => break,
                }
            }
        });
    }
}
