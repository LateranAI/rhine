// 标准库
use std::sync::Arc;

// 并发和同步原语
use dashmap::DashMap;
use once_cell::sync::Lazy;
use tokio::sync::Semaphore;

// HTTP客户端
use reqwest::Client;

// 错误处理
use error_stack::Result;
use thiserror::Error;

/// 配置相关错误枚举
/// Configuration related error enum
#[derive(Debug, Error)]
pub enum ConfigError {
    /// 获取配置锁失败
    /// Failed to acquire configuration lock
    #[error("Failed to acquire config lock")]
    ConfigLockFailure,
    
    /// 配置未初始化
    /// Configuration not initialized
    #[error("Configuration not initialized")]
    ConfigNotInitialized,
    
    /// API信息未找到
    /// API information not found
    #[error("API info not found")]
    ApiInfoNotFound,
}

/// 模型能力枚举
/// Model capability enum
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ModelCapability {
    /// 思考能力
    /// Thinking capability
    Think,
    
    /// 工具使用能力
    /// Tool usage capability
    ToolUse,
    
    /// 长上下文处理能力
    /// Long context processing capability
    LongContext,
}

/// API来源结构体
/// API source structure
#[derive(Clone, Debug)]
pub struct ApiSource {
    /// API基础URL
    /// API base URL
    pub base_url: String,
    
    /// 并行请求数量限制
    /// Parallel request limit
    pub parallelism: usize,
}

/// API信息结构体
/// API information structure
#[derive(Clone, Debug)]
pub struct ApiInfo {
    /// 模型名称
    /// Model name
    pub model: String,
    
    /// API基础URL
    /// API base URL
    pub base_url: String,
    
    /// API密钥
    /// API key
    pub api_key: String,
    
    /// HTTP客户端实例
    /// HTTP client instance
    pub client: Client,
}

/// 配置管理结构体
/// Configuration management structure
#[derive(Clone, Debug)]
pub struct Config {
    /// API来源映射表 - 存储名称到API来源的映射
    /// API source map - stores mappings from name to API source
    pub api_source: DashMap<String, ApiSource>,
    
    /// API信息映射表 - 存储(名称,能力)到API信息的映射
    /// API info map - stores mappings from (name, capability) to API info
    pub api_info: DashMap<(String, ModelCapability), ApiInfo>,
}

impl Config {
    /// 添加API来源
    /// Add API source
    ///
    /// # 参数 (Parameters)
    /// * `name` - API来源名称
    ///          - API source name
    /// * `base_url` - API基础URL
    ///              - API base URL
    /// * `parallelism` - 并行度（允许的并发请求数）
    ///                 - Parallelism (allowed concurrent requests)
    pub fn add_api_source(name: &str, base_url: &str, parallelism: usize) {
        // 向配置中添加API来源
        // Add API source to configuration
        CFG.api_source.insert(
            name.to_string(),
            ApiSource {
                base_url: base_url.to_string(),
                parallelism,
            },
        );

        // 为该API来源创建信号量用于控制并发
        // Create semaphore for this API source to control concurrency
        THREAD_POOL.insert(base_url.to_string(), Arc::new(Semaphore::new(parallelism)));
    }

    /// 添加API信息
    /// Add API information
    ///
    /// # 参数 (Parameters)
    /// * `name` - API名称
    ///          - API name
    /// * `model` - 模型名称
    ///           - Model name
    /// * `capability` - 模型能力
    ///                - Model capability
    /// * `source_name` - API来源名称
    ///                 - API source name
    /// * `api_key` - API密钥
    ///             - API key
    pub fn add_api_info(
        name: &str,
        model: &str,
        capability: ModelCapability,
        source_name: &str,
        api_key: &str,
    ) {
        // 获取API来源的基础URL
        // Get the base URL of API source
        let base_url = CFG
            .api_source
            .get(source_name)
            .unwrap()
            .base_url
            .clone();
        
        // 向配置中添加API信息
        // Add API information to configuration
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

    /// 根据名称获取API信息
    /// Get API information by name
    ///
    /// # 参数 (Parameters)
    /// * `name` - API名称
    ///          - API name
    ///
    /// # 返回 (Returns)
    /// * `Result<ApiInfo, ConfigError>` - 成功返回API信息，失败返回配置错误
    ///                                  - Returns API info on success, config error on failure
    pub fn get_api_info_with_name(name: String) -> Result<ApiInfo, ConfigError> {
        // 在API信息映射表中查找匹配的条目
        // Find matching entry in API info map
        CFG.api_info
            .iter()
            .find_map(|entry| {
                (entry.key().0 == name).then(|| entry.value().clone())
            })
            .ok_or(ConfigError::ApiInfoNotFound.into())
    }

    /// 根据模型能力获取API信息
    /// Get API information by model capability
    ///
    /// # 参数 (Parameters)
    /// * `capability` - 模型能力
    ///                - Model capability
    ///
    /// # 返回 (Returns)
    /// * `Result<ApiInfo, ConfigError>` - 成功返回API信息，失败返回配置错误
    ///                                  - Returns API info on success, config error on failure
    pub fn get_api_info_with_capability(
        capability: ModelCapability,
    ) -> Result<ApiInfo, ConfigError> {
        // 在API信息映射表中查找匹配的条目
        // Find matching entry in API info map
        CFG.api_info
            .iter()
            .find_map(|entry| {
                (entry.key().1 == capability).then(|| entry.value().clone())
            })
            .ok_or(ConfigError::ApiInfoNotFound.into())
    }
}

/// 全局配置实例
/// Global configuration instance
pub static CFG: Lazy<Config> = Lazy::new(|| {
    Config {
        api_source: DashMap::new(),
        api_info: DashMap::new(),
    }
});

/// 全局线程池（信号量池）- 用于控制对不同API来源的并发请求
/// Global thread pool (semaphore pool) - used to control concurrent requests to different API sources
pub static THREAD_POOL: Lazy<DashMap<String, Arc<Semaphore>>> = Lazy::new(|| DashMap::new());