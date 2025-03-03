use error_stack::{Result, ResultExt};  // 引入 error-stack
use dashmap::DashMap;
use once_cell::sync::OnceCell;
use regex::Regex;
use std::sync::Arc;
use thiserror::Error;
use crate::chat::chat_tool::ChatTool;
// 引入 thiserror

// 定义错误类型
#[derive(Debug, Error)]
pub enum ChatToolSchemaError {
    #[error("Failed to assemble tool prompt")]
    AssembleToolPrompt,
    #[error("Missing 'function' field")]
    MissingFunctionField,
    #[error("Missing or invalid 'function.name' field")]
    MissingFunctionName,
    #[error("Missing or invalid 'function.description' field")]
    MissingFunctionDescription,
    #[error("Missing 'function.parameters' field")]
    MissingFunctionParameters,
    #[error("Missing 'function.parameters.properties' field")]
    MissingFunctionProperties,
    #[error("Failed to parse params {1} for function: {0}")]
    ParamsParseError(String, String),
    #[error("Failed to parse result for function: {0}")]
    ResultParseError(String),
    #[error("Failed to call function")]
    FunctionCallError,
}

// 修改 ToolFunction 类型定义，使用 error_stack::Result
type ToolFunction = Arc<dyn Fn(serde_json::Value) -> Result<serde_json::Value, ChatToolSchemaError> + Send + Sync>;

static REGISTRY: OnceCell<DashMap<String, ToolFunction>> = OnceCell::new();


pub fn create_tool(
    name: &str,
    func: impl Fn(serde_json::Value) -> Result<serde_json::Value, ChatToolSchemaError> + Send + Sync + 'static,
) -> (String, ToolFunction) {
    (name.to_string(), Arc::new(func))
}

pub fn get_tool_registry() -> &'static DashMap<String, ToolFunction> {
    REGISTRY.get_or_init(|| DashMap::new())
}

pub fn get_tool_function(name: &str) -> Option<ToolFunction> {
    get_tool_registry().get(name).map(|entry| entry.value().clone())
}

pub async fn tool_use(text_answer: &str, tools_schema: serde_json::Value) -> Result<(), ChatToolSchemaError> {
    let functions_calling = extract_tool_uses(text_answer);
    for function_calling in functions_calling {
        ChatTool::get_function(function_calling.as_str(), tools_schema.clone()).await
            .change_context(ChatToolSchemaError::FunctionCallError)?; // 使用 change_context 转换错误
    }
    Ok(())
}

pub fn extract_tool_uses(input: &str) -> Vec<String> {
    // 定义正则表达式，匹配 <ToolUse> 标签包裹的内容，支持多行
    let re = Regex::new(r"(?s)<ToolUse>(.*?)</ToolUse>").unwrap();

    re.captures_iter(input)
        .map(|cap| cap[1].trim().to_string())
        .collect()
}