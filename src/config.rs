use error_stack::Result;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
    pub api_source: HashMap<String, ApiSource>,
    pub api_info: HashMap<(String, ModelCapability), ApiInfo>,
}

impl Config {
    pub fn add_api_source(name: String, base_url: String, parallelism: usize) {
        let mut cfg_lock = CFG.lock().unwrap();
        let mut cfg_clone = cfg_lock.clone().unwrap();

        cfg_clone.api_source.insert(
            name.clone(),
            ApiSource {
                base_url,
                parallelism,
            },
        );
        *cfg_lock = Some(cfg_clone);

        let mut thread_pool_lock = THREAD_POOL.lock().unwrap();
        let mut thread_pool_clone = thread_pool_lock.clone();
        thread_pool_clone.insert(name, Arc::new(Semaphore::new(parallelism)));
        *thread_pool_lock = thread_pool_clone;
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
        let base_url = cfg_clone
            .api_source
            .get(&sourse_name)
            .unwrap()
            .base_url
            .clone();
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

    pub fn get_api_info_with_name(name: String) -> Result<ApiInfo, ConfigError> {
        let cfg_lock = CFG.lock().map_err(|_| ConfigError::ConfigLockFailure)?;

        // 处理配置未初始化错误
        let cfg = cfg_lock.as_ref().ok_or(ConfigError::ConfigNotInitialized)?;

        // 处理API信息不存在错误
        cfg.api_info
            .iter()
            .find_map(|((n, _), v)| (n == &name).then(|| v.clone()))
            .ok_or(ConfigError::ApiInfoNotFound.into())
    }

    pub fn get_api_info_with_capablity(
        capability: ModelCapability,
    ) -> Result<ApiInfo, ConfigError> {
        let cfg_lock = CFG.lock().map_err(|_| ConfigError::ConfigLockFailure)?;

        // 处理配置未初始化错误
        let cfg = cfg_lock.as_ref().ok_or(ConfigError::ConfigNotInitialized)?;

        // 处理API信息不存在错误
        cfg.api_info
            .iter()
            .find_map(|((_, c), v)| (c == &capability).then(|| v.clone()))
            .ok_or(ConfigError::ApiInfoNotFound.into())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiSource {
    pub base_url: String,
    pub parallelism: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiInfo {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ModelCapability {
    Think,
    ToolUse,
    LongContext,
}

pub static CFG: Lazy<Mutex<Option<Config>>> = Lazy::new(|| Mutex::new(None));
pub static THREAD_POOL: Lazy<Mutex<HashMap<String, Arc<Semaphore>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
