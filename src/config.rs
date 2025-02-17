use error_stack::Result;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use dashmap::DashMap;
use reqwest::Client;
use thiserror::Error;
use tokio::sync::Semaphore;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to acquire config lock")]
    ConfigLockFailure,
    #[error("Configuration not initialized")]
    ConfigNotInitialized,
    #[error("API info not found")]
    ApiInfoNotFound,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub api_source: DashMap<String, ApiSource>,
    pub api_info: DashMap<(String, ModelCapability), ApiInfo>,
}

impl Config {
    pub fn add_api_source(name: &str, base_url: &str, parallelism: usize) {
        CFG.api_source.insert(
            name.to_string(),
            ApiSource {
                base_url: base_url.to_string(),
                parallelism,
            },
        );

        THREAD_POOL.insert(base_url.to_string(), Arc::new(Semaphore::new(parallelism)));
    }

    pub fn add_api_info(
        name: &str,
        model: &str,
        capability: ModelCapability,
        source_name: &str,
        api_key: &str,
    ) {
        let base_url = CFG
            .api_source
            .get(source_name)
            .unwrap()
            .base_url
            .clone();
        CFG.api_info.insert(
            (name.to_string(), capability),
            ApiInfo {
                model: model.to_string(),
                base_url,
                api_key: api_key.to_string(),
                client: Client::new(),
            },
        );
    }

    pub fn get_api_info_with_name(name: String) -> Result<ApiInfo, ConfigError> {
        CFG.api_info
            .iter()
            .find_map(|entry| {
                (entry.key().0 == name).then(|| entry.value().clone())
            })
            .ok_or(ConfigError::ApiInfoNotFound.into())
    }

    pub fn get_api_info_with_capability(
        capability: ModelCapability,
    ) -> Result<ApiInfo, ConfigError> {
        CFG.api_info
            .iter()
            .find_map(|entry| {
                (entry.key().1 == capability).then(|| entry.value().clone())
            })
            .ok_or(ConfigError::ApiInfoNotFound.into())
    }

}

#[derive(Clone, Debug)]
pub struct ApiSource {
    pub base_url: String,
    pub parallelism: usize,
}

#[derive(Clone, Debug)]
pub struct ApiInfo {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub client: Client,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ModelCapability {
    Think,
    ToolUse,
    LongContext,
}

pub static CFG: Lazy<Config> = Lazy::new(|| {
    Config {
        api_source: DashMap::new(),
        api_info: DashMap::new(),
    }
});

pub static THREAD_POOL: Lazy<DashMap<String, Arc<Semaphore>>> = Lazy::new(|| DashMap::new());
