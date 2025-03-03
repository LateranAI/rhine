// 错误处理和结果类型
use error_stack::{Report, Result, ResultExt};
// 序列化相关
use serde::de::DeserializeOwned;
// 日志功能
use tracing::log::info;

// 项目内部模块
use crate::chat::chat_base::{BaseChat, ChatError, Role};
use crate::config::ModelCapability::ToolUse;
use crate::schema::json_schema::JsonSchema;

/// ChatTool结构体：提供与语言模型交互的工具功能
/// ChatTool struct: Provides utility functions for interacting with language models
pub struct ChatTool;

impl ChatTool {
    /// 从文本获取JSON格式的结果
    /// Get JSON formatted result from text input
    ///
    /// # 参数 (Parameters)
    /// * `text_answer` - 需要转换为JSON的文本输入
    ///                 - Text input to be converted to JSON
    /// * `json_schema` - 定义输出JSON格式的模式
    ///                 - Schema defining the output JSON format
    ///
    /// # 返回 (Returns)
    /// * `Result<T, ChatError>` - 成功时返回反序列化的T类型数据，失败时返回ChatError
    ///                          - Returns deserialized data of type T on success, ChatError on failure
    pub async fn get_json<T: DeserializeOwned + 'static + JsonSchema>(
        text_answer: &str,
        json_schema: serde_json::Value,
    ) -> Result<T, ChatError> {
        // 创建支持工具使用能力的基础聊天实例
        // Create a base chat instance with tool use capability
        let mut base = BaseChat::new_with_model_capability(
            ToolUse,
            "将输入内容整理为指定的json形式输出", // Format input content into specified JSON output
            false,
        );

        // 添加用户消息
        // Add user message
        base.add_message(Role::User, text_answer);

        // 构建包含响应格式的请求体
        // Build request body with response format
        let request_body = add_response_format(base.build_request_body(), json_schema);

        // 发送请求并处理可能的错误
        // Send request and handle potential errors
        let response = base.get_response(request_body)
            .await
            .change_context(ChatError::GetJsonError)
            .attach_printable("Failed to send request")?;

        // 从响应中提取内容
        // Extract content from response
        let json_answer = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(Report::new(ChatError::GetJsonError))
            .attach_printable("Failed to get content from response")?;

        // 记录LLM返回的答案
        // Log the answer from LLM
        info!("Get LLM API Answer: {}", json_answer);

        // 添加助手回复
        // Add assistant reply
        base.add_message(Role::Assistant, json_answer);

        // 将JSON字符串反序列化为目标类型
        // Deserialize JSON string to target type
        serde_json::from_str(json_answer)
            .change_context(ChatError::GetJsonError)
            .attach_printable_lazy(|| format!("Failed to deserialize JSON: {}", json_answer))
    }

    /// 基于输入文本调用函数
    /// Call a function based on text input
    ///
    /// # 参数 (Parameters)
    /// * `text_answer` - 用户输入的文本
    ///                 - Text input from user
    /// * `tools_schema` - 可用工具的模式定义
    ///                  - Schema defining available tools
    ///
    /// # 返回 (Returns)
    /// * `Result<serde_json::Value, ChatError>` - 成功时返回函数调用的JSON结果，失败时返回ChatError
    ///                                          - Returns JSON result of function call on success, ChatError on failure
    pub async fn get_function(
        text_answer: &str,
        tools_schema: serde_json::Value,
    ) -> Result<serde_json::Value, ChatError> {
        // 创建支持工具使用能力的基础聊天实例
        // Create a base chat instance with tool use capability
        let mut base = BaseChat::new_with_model_capability(
            ToolUse,
            "根据输入的内容调用指定的函数", // Call specified function based on input content
            false,
        );

        // 添加用户消息
        // Add user message
        base.add_message(Role::User, text_answer);

        // 构建包含工具的请求体
        // Build request body with tools
        let request_body = add_tools(base.build_request_body(), tools_schema);

        // 发送请求并处理可能的错误
        // Send request and handle potential errors
        let response = base.get_response(request_body)
            .await
            .change_context(ChatError::GetFunctionError)
            .attach_printable("Failed to send request")?;

        // 从响应中提取函数调用结果
        // Extract function call result from response
        let json_answer = response["choices"][0]["message"]["tool_calls"][0]["function"].clone();

        Ok(json_answer)
    }
}

/// 向请求体添加响应格式配置
/// Add response format configuration to request body
///
/// # 参数 (Parameters)
/// * `request_body` - 原始请求体
///                  - Original request body
/// * `schema` - JSON模式定义
///            - JSON schema definition
///
/// # 返回 (Returns)
/// * `serde_json::Value` - 添加了响应格式后的请求体
///                       - Request body with response format added
fn add_response_format(
    mut request_body: serde_json::Value,
    schema: serde_json::Value,
) -> serde_json::Value {
    // 创建响应格式配置
    // Create response format configuration
    let response_format = serde_json::json!({
        "response_format": schema
    });

    // 将响应格式添加到请求体中
    // Add response format to request body
    if let serde_json::Value::Object(ref mut body) = request_body {
        if let serde_json::Value::Object(format) = response_format {
            body.extend(format);
        }
    }
    request_body
}

/// 向请求体添加工具配置
/// Add tools configuration to request body
///
/// # 参数 (Parameters)
/// * `request_body` - 原始请求体
///                  - Original request body
/// * `schema` - 工具模式定义
///            - Tools schema definition
///
/// # 返回 (Returns)
/// * `serde_json::Value` - 添加了工具配置后的请求体
///                       - Request body with tools configuration added
fn add_tools(
    mut request_body: serde_json::Value,
    schema: serde_json::Value
) -> serde_json::Value {
    // 将工具配置添加到请求体中
    // Add tools configuration to request body
    if let serde_json::Value::Object(ref mut body) = request_body {
        if let serde_json::Value::Object(format) = schema {
            body.extend(format);
        }
    }
    request_body
}