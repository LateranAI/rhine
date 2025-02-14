use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct Config {
    pub api_source: HashMap<String, ApiSource>,
    pub api_info: HashMap<(String, ModelCapability), ApiInfo>,
}

impl Config {
    pub fn add_api_source(name: String, base_url: String, parallelism: u8) {
        let mut cfg_lock = CFG.lock().unwrap();
        let mut cfg_clone = cfg_lock.clone().unwrap();

        cfg_clone.api_source.insert(
            name,
            ApiSource {
                base_url,
                parallelism,
            },
        );
        *cfg_lock = Some(cfg_clone);
    }

    pub fn add_api_info(
        name: String,
        model: String,
        capability: ModelCapability,
        sourse_name: String,
        api_key: String,
    ) {
        let mut cfg_lock = CFG.lock().unwrap();
        let mut cfg_clone = cfg_lock.clone().unwrap();
        let base_url = cfg_clone.api_source.get(&sourse_name).unwrap().base_url.clone();
        cfg_clone.api_info.insert(
            (name.clone(), capability),
            ApiInfo {
                model,
                base_url,
                api_key,
            },
        );
        *cfg_lock = Some(cfg_clone);
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiSource {
    pub base_url: String,
    pub parallelism: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiInfo {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

pub enum ModelCapability {
    Think,
    ToolUse,
    LongContext,
}

static CFG: Lazy<Mutex<Option<Config>>> = Lazy::new(|| Mutex::new(None));
