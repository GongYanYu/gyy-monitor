use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use crate::config::Config;

pub struct AppState {
    pub config: Mutex<Config>,
    pub metrics: Mutex<serde_json::Value>,
    pub is_game_active: AtomicBool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        AppState {
            config: Mutex::new(config),
            metrics: Mutex::new(serde_json::json!({})),
            is_game_active: AtomicBool::new(false),
        }
    }
}
