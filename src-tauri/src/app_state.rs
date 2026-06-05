use std::sync::Mutex;
use crate::config::Config;

pub struct AppState {
    pub config: Mutex<Config>,
    pub metrics: Mutex<serde_json::Value>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        AppState {
            config: Mutex::new(config),
            metrics: Mutex::new(serde_json::json!({})),
        }
    }
}
