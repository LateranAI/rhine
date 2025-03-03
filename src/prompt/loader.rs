// 标准库
use std::collections::HashMap;

// 错误处理
use error_stack::{Result, ResultExt};
use thiserror::Error;

// 项目内部模块
use crate::prompt::model::{Config, Content, Info, Template};
use crate::utils::common::load_toml::load_toml;

/// 提示加载错误枚举
/// Prompt loading error enum
#[derive(Debug, Error)]
pub enum PromptLoadError {
    /// 配置加载失败
    /// Failed to load configuration
    #[error("Failed to load config")]
    ConfigLoadError,
    
    /// 模板加载失败
    /// Failed to load template
    #[error("Failed to load template")]
    TemplateLoadError,
    
    /// 内容加载失败
    /// Failed to load content
    #[error("Failed to load content for {0}")]
    ContentLoadError(String),
}

/// 加载提示模板和内容
/// Load prompt templates and contents
///
/// # 返回 (Returns)
/// * `Result<(Template, HashMap<Info, Content>), PromptLoadError>` - 成功返回模板和内容映射，失败返回错误
///                                                                 - Returns template and content mapping on success, error on failure
pub fn load() -> Result<(Template, HashMap<Info, Content>), PromptLoadError> {
    // 加载配置
    // Load configuration
    let config: Config = load_toml("data/prompts/config.toml")
        .change_context(PromptLoadError::ConfigLoadError)?;
    
    // 加载模板
    // Load template
    let template: Template = load_toml(&config.template_path)
        .change_context(PromptLoadError::TemplateLoadError)?;

    // 预分配容量减少重新分配
    // Pre-allocate capacity to reduce reallocations
    let mut info_with_contents = HashMap::with_capacity(config.prompt_info.len());
    
    // 加载每个信息对应的内容
    // Load content for each info
    for info in &config.prompt_info {
        let content: Content = load_toml(&info.path)
            .change_context_lazy(|| PromptLoadError::ContentLoadError(info.name.clone()))?;
        
        info_with_contents.insert(info.clone(), content);
    }

    Ok((template, info_with_contents))
}

/// 加载提示模板和内容（无错误处理版本，保持向后兼容）
/// Load prompt templates and contents (no error handling version, for backward compatibility)
///
/// # 返回 (Returns)
/// * `(Template, HashMap<Info, Content>)` - 模板和信息内容映射的元组
///                                        - Tuple of template and information content mapping
///
/// # 注意 (Note)
/// 如果加载过程中出现错误，此函数将会panic
/// This function will panic if there's an error during loading
#[deprecated(since = "next_version", note = "请使用返回Result的load函数代替")]
pub fn load_unchecked() -> (Template, HashMap<Info, Content>) {
    // 加载配置
    // Load configuration
    let config: Config = load_toml("data/prompts/config.toml")
        .expect("Failed to load config.toml");
    
    // 加载模板
    // Load template
    let template: Template = load_toml(&config.template_path)
        .expect(&format!("Failed to load template from {}", &config.template_path));

    // 预分配容量减少重新分配
    // Pre-allocate capacity to reduce reallocations
    let mut info_with_contents = HashMap::with_capacity(config.prompt_info.len());
    
    // 加载每个信息对应的内容
    // Load content for each info
    for info in &config.prompt_info {
        let content: Content = load_toml(&info.path)
            .expect(&format!("Failed to load content from {}", &info.path));
        
        info_with_contents.insert(info.clone(), content);
    }

    (template, info_with_contents)
}