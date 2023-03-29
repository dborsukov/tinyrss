use super::utils;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{io::Write, sync::Arc};
use tracing::{error, info};

lazy_static! {
    pub static ref CONFIG: Arc<Mutex<ConfigBuilder>> =
        Arc::new(Mutex::new(ConfigBuilder::from_file()));
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfigBuilder {
    pub show_search_in_feed: bool,
    pub auto_dismiss_on_open: bool,
    pub max_allowed_concurent_requests: usize,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            show_search_in_feed: false,
            auto_dismiss_on_open: false,
            max_allowed_concurent_requests: 5,
        }
    }
}

impl ConfigBuilder {
    pub fn from_current() -> Self {
        CONFIG.lock().clone()
    }

    pub fn from_file() -> Self {
        let app_dir = utils::get_app_dir();
        let config_path = app_dir.join("config.yml");

        match std::fs::File::open(config_path) {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                match serde_yaml::from_reader(reader) {
                    Ok(loaded_config) => {
                        info!("Successfuly loaded application config from file.");
                        return loaded_config;
                    }
                    Err(err) => {
                        error!("Failed to deserialize config file: {}", err.to_string());
                        info!("Using default config file.");
                        return Self::default();
                    }
                }
            }
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    error!("Error kind: {}", err.kind());
                    error!("Failed to read config file: {}", err.to_string());
                }
                info!("Using default config file.");
                return Self::default();
            }
        }
    }

    pub fn apply(self) {
        let mut temp = CONFIG.lock();
        *temp = self;
    }

    pub fn save(self) -> Result<(), Box<dyn std::error::Error>> {
        let app_dir = utils::get_app_dir();
        let config_path = app_dir.join("config.yml");

        let yaml = serde_yaml::to_string(&self)?;
        let mut file = std::fs::File::create(config_path)?;

        file.write_all(yaml.as_bytes())?;

        Ok(())
    }
}
