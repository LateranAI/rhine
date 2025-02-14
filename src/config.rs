use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::Deserialize;

pub struct Config {
    pub chat: HashMap<(String, ChatModelAttr), ChatApiInfo>
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChatApiInfo {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

pub enum ChatModelAttr {
    Think,
    ToolUse,
    LongContext,
}

pub fn add_chat_api_info(
    name: String,
    model: String,
    attribute: ChatModelAttr,
    base_url: String,
    api_key: String,
) {
    let mut cfg_lock = CFG.lock().unwrap();
    let mut cfg_clone = cfg_lock.clone().unwrap();
    cfg_clone.chat.insert(
        (name.clone(), attribute),
        ChatApiInfo {
            model,
            base_url,
            api_key,
        },
    );
    *cfg_lock = Some(cfg_clone);
}

static CFG: Lazy<Mutex<Option<Config>>> =
    Lazy::new(|| Mutex::new(None));


pub fn set_config(config: Config) {
    let mut config_lock = CONFIG.lock().unwrap();
    *config_lock = Some(config);
}

pub fn get_config() -> Option<Config> {
    let config_lock = CONFIG.lock().unwrap();
    config_lock.clone()
}