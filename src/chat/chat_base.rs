// 标准库引用 / Standard library imports
use std::collections::HashMap;

// 外部库引用 / External library imports (按泛用程度从高到低排序 / ordered by generality from high to low)
// 基础数据类型和序列化 / Basic data types and serialization
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;

// 错误处理 / Error handling
use error_stack::{Report, Result, ResultExt};
use thiserror::Error;

// 异步运行时和流处理 / Async runtime and stream processing
use futures::{Stream, TryStreamExt};
use tokio::sync::OwnedSemaphorePermit;

// 网络请求 / Network requests
use crate::chat::message::{Messages, Role};
use reqwest::{Client, Error, Response};
use tracing::info;
// 本地库引用 / Local library imports
use crate::config::{Config, ModelCapability, THREAD_POOL};

/// 聊天相关错误枚举
/// Chat related error enumeration
#[derive(Debug, Error)]
pub enum ChatError {
    // 提示相关错误 / Prompt related errors
    /// 组装输出描述失败
    /// Failed to assemble output description
    #[error("Failed to assemble output description")]
    AssembleOutputDescriptionError,

    // HTTP 连接错误 / HTTP connection errors
    /// HTTP 错误，包含状态码
    /// HTTP error with status code
    #[error("HTTP error with status code: {0}")]
    HttpError(u16),
    /// 超时错误
    /// Timeout error
    #[error("Timeout error")]
    TimeoutError,

    // 结果解析错误 / Result parsing errors
    /// 解析响应失败
    /// Failed to parse response
    #[error("Failed to parse response")]
    ParseResponseError,
    /// 缺少使用量数据
    /// Missing usage data
    #[error("Missing usage data")]
    MissingUsageData,

    // 工具使用错误 / Tool usage errors
    /// 获取 JSON 失败
    /// Failed to get JSON
    #[error("Failed to get json")]
    GetJsonError,
    /// 获取函数失败
    /// Failed to get function
    #[error("Failed to get function")]
    GetFunctionError,

    /// 没有提供角色提示词
    /// No character prompts provided
    #[error("At least one character prompt required")]
    NoCharacterPrompts,
    /// 未定义的角色
    /// Undefined character
    #[error("Undefined character: {0}")]
    UndefinedCharacter(String),
    /// 未选择角色
    /// No character selected
    #[error("No character selected")]
    NoCharacterSelected,

    /// 未知错误
    /// Unknown error
    #[error("Unknown error")]
    UnknownError,
}

/// 基础聊天结构体，用于与 AI 对话服务交互
/// Base chat structure for interacting with AI conversation services
#[derive(Debug, Clone)]
pub struct BaseChat {
    /// 模型名称
    /// Model name
    pub model: String,
    /// 基础 URL
    /// Base URL
    pub base_url: String,
    /// API 密钥
    /// API key
    pub api_key: String,
    /// HTTP 客户端
    /// HTTP client
    pub client: Client,
    /// 角色提示词
    /// Character prompt
    pub character_prompt: String,
    /// 消息路径
    /// Message Path
    pub message_path: Vec<usize>,
    /// 消息树
    /// Message Tree
    pub messages: Option<Messages>,
    /// Token 使用量
    /// Token usage
    pub usage: i32,
    /// 是否需要流式响应
    /// Whether streaming response is needed
    pub need_stream: bool,
}

impl BaseChat {
    /// 使用 API 名称创建新的聊天实例
    ///
    /// Create a new chat instance with API name
    ///
    /// # 参数 / Parameters
    /// * `api_name` - API 名称 / API name
    /// * `character_prompt` - 角色提示词 / Character prompt
    /// * `need_stream` - 是否需要流式响应 / Whether streaming response is needed
    ///
    /// # 返回 / Returns
    /// * `Self` - 新创建的 BaseChat 实例 / Newly created BaseChat instance
    pub fn new_with_api_name(api_name: &str, character_prompt: &str, need_stream: bool) -> Self {
        let api_info = Config::get_api_info_with_name(api_name.to_string()).unwrap();

        Self {
            model: api_info.model,
            base_url: api_info.base_url,
            api_key: api_info.api_key,
            client: api_info.client,
            character_prompt: character_prompt.to_string(),
            message_path: vec![],
            messages: None,
            usage: 0,
            need_stream,
        }
    }

    /// 使用模型能力创建新的聊天实例
    ///
    /// Create a new chat instance with model capability
    ///
    /// # 参数 / Parameters
    /// * `model_capability` - 模型能力枚举 / Model capability enum
    /// * `character_prompt` - 角色提示词 / Character prompt
    /// * `need_stream` - 是否需要流式响应 / Whether streaming response is needed
    ///
    /// # 返回 / Returns
    /// * `Self` - 新创建的 BaseChat 实例 / Newly created BaseChat instance
    pub fn new_with_model_capability(
        model_capability: ModelCapability,
        character_prompt: &str,
        need_stream: bool,
    ) -> Self {
        let api_info = Config::get_api_info_with_capability(model_capability.clone()).unwrap();

        Self {
            model: api_info.model,
            base_url: api_info.base_url,
            api_key: api_info.api_key,
            client: api_info.client,
            character_prompt: character_prompt.to_string(),
            message_path: vec![],
            messages: None,
            usage: 0,
            need_stream,
        }
    }

    /// 添加消息到消息列表
    ///
    /// Add a message to the message list
    ///
    /// # 参数 / Parameters
    /// * `role` - 消息角色 / Message role
    /// * `content` - 消息内容 / Message content
    pub fn add_message(&mut self, role: Role, content: &str) {
        if let Some(messages) = &mut self.messages {
            messages
                .add(self.message_path.as_ref(), role, content.to_string())
                .unwrap();
        } else {
            self.messages = Some(Messages::new(role, content.to_string()))
        }
    }

    /// 构建请求体
    ///
    /// Build request body
    ///
    /// # 返回 / Returns
    /// * `serde_json::Value` - JSON 格式的请求体 / Request body in JSON format
    pub fn build_request_body(
        &self,
        end_path: &[usize],
        current_speaker: &Role,
    ) -> serde_json::Value {
        let Some(messages) = self.messages.as_ref() else {
            return json!({
                "model": self.model,
                "messages": [],
                "stream": self.need_stream,
            });
        };
        let messages = messages.assemble_context([].as_ref(), end_path, current_speaker);

        json!({
            "model": self.model,
            "messages": messages,
            "stream": self.need_stream,
        })
    }

    /// 发送 HTTP 请求
    ///
    /// Send HTTP request
    ///
    /// # 参数 / Parameters
    /// * `request_body` - 请求体 / Request body
    ///
    /// # 返回 / Returns
    /// * `core::result::Result<Response, Error>` - HTTP 响应结果 / HTTP response result
    pub async fn send_request(
        &mut self,
        request_body: serde_json::Value,
    ) -> core::result::Result<Response, Error> {
        self.client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .bearer_auth(&self.api_key)
            .json(&request_body)
            // .timeout(Duration::from_secs(5))  // 启用此行可添加超时设置 / Uncomment this line to add timeout
            .send()
            .await
    }

    /// 获取 API 响应
    ///
    /// Get API response
    ///
    /// # 参数 / Parameters
    /// * `request_body` - 请求体 / Request body
    ///
    /// # 返回 / Returns
    /// * `Result<serde_json::Value, ChatError>` - API 响应结果 / API response result
    pub async fn get_response(
        &mut self,
        request_body: serde_json::Value,
    ) -> Result<serde_json::Value, ChatError> {
        // 获取信号量许可
        // Acquire semaphore permit
        let semaphore_permit = THREAD_POOL
            .get(&self.base_url)
            .unwrap()
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        // 发送请求
        // Send request
        let response = self.send_request(request_body.clone()).await;

        // 释放信号量许可
        // Release semaphore permit
        drop(semaphore_permit);

        match response {
            Ok(res) => {
                // 处理 HTTP 状态码错误
                // Handle HTTP status code errors
                let res = res.error_for_status().map_err(|e| {
                    Report::new(ChatError::HttpError(e.status().unwrap().as_u16()))
                        .attach_printable(format!("HTTP error with request body: {}", request_body))
                })?;

                // 解析 JSON 响应
                // Parse JSON response
                let parsed: serde_json::Value = res
                    .json()
                    .await
                    .change_context(ChatError::ParseResponseError)
                    .attach_printable("Failed to parse response JSON")?;

                // 更新 token 使用量
                // Update token usage
                self.usage += parsed["usage"]["total_tokens"]
                    .as_i64()
                    .ok_or_else(|| Report::new(ChatError::MissingUsageData))
                    .attach_printable("Missing usage data in response")?
                    as i32;

                Ok(parsed)
            }
            Err(e) => {
                if e.is_timeout() {
                    Err(Report::new(ChatError::TimeoutError)
                        .attach_printable(format!("Request timeout: {}", request_body)))
                } else {
                    Err(Report::new(ChatError::UnknownError)
                        .attach_printable(format!("Network error: {} - {}", e, request_body)))
                }
            }
        }
    }

    /// 从响应中提取内容
    ///
    /// Extract content from response
    ///
    /// # 参数 / Parameters
    /// * `resp` - API 响应 / API response
    ///
    /// # 返回 / Returns
    /// * `Result<String, ChatError>` - 提取的内容 / Extracted content
    pub fn get_content_from_resp(resp: &serde_json::Value) -> Result<String, ChatError> {
        let content = resp
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"));

        match content {
            Some(content) => Ok(content.to_string()),
            None => Err(Report::new(ChatError::ParseResponseError))
                .attach_printable("Failed to parse response content"),
        }
    }

    /// 获取流式响应
    ///
    /// Get streaming response
    ///
    /// # 参数 / Parameters
    /// * `request_body` - 请求体 / Request body
    ///
    /// # 返回 / Returns
    /// * `Result<(impl Stream<Item=reqwest::Result<Bytes>> + Send + Unpin, OwnedSemaphorePermit), ChatError>` -
    ///   字节流和信号量许可 / Byte stream and semaphore permit
    pub async fn get_stream_response(
        &mut self,
        request_body: serde_json::Value,
    ) -> Result<
        (
            impl Stream<Item = reqwest::Result<Bytes>> + Send + Unpin,
            OwnedSemaphorePermit,
        ),
        ChatError,
    > {
        // 获取信号量许可
        // Acquire semaphore permit
        let semaphore_permit = THREAD_POOL
            .get(&self.base_url)
            .unwrap()
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        // 发送请求
        // Send request
        let response = self.send_request(request_body.clone()).await;

        match response {
            Ok(res) => {
                // 处理 HTTP 状态码错误
                // Handle HTTP status code errors
                let res = res.error_for_status().map_err(|e| {
                    Report::new(ChatError::HttpError(e.status().unwrap().as_u16()))
                        .attach_printable(format!("HTTP error with request body: {}", request_body))
                })?;

                Ok((res.bytes_stream(), semaphore_permit))
            }
            Err(e) => {
                if e.is_timeout() {
                    Err(Report::new(ChatError::TimeoutError)
                        .attach_printable(format!("Request timeout: {}", request_body)))
                } else {
                    Err(Report::new(ChatError::UnknownError)
                        .attach_printable(format!("Network error: {} - {}", e, request_body)))
                }
            }
        }
    }

    /// 从流式响应中提取内容
    ///
    /// Extract content from streaming response
    ///
    /// # 参数 / Parameters
    /// * `stream` - 字节流 / Byte stream
    /// * `semaphore_permit` - 信号量许可 / Semaphore permit
    ///
    /// # 返回 / Returns
    /// * `Result<String, ChatError>` - 提取的内容 / Extracted content
    pub async fn get_content_from_stream_resp(
        stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + Unpin,
        semaphore_permit: OwnedSemaphorePermit,
    ) -> Result<String, ChatError> {
        // 创建用于收集结果的结构
        // Create structure for collecting results
        #[derive(Default)]
        struct StreamResult {
            content: String,
            usage: Option<serde_json::Value>,
        }

        let result = stream
            .map_err(|err| {
                Report::new(ChatError::HttpError(0))
                    .attach_printable(format!("Failed to get response: {}", err))
            })
            .try_fold(StreamResult::default(), |mut result, chunk| async move {
                String::from_utf8_lossy(&chunk)
                    .split('\n')
                    .filter(|line| !line.is_empty() && *line != "data: [DONE]")
                    .try_for_each(|line| {
                        // 移除可能的 "data: " 前缀 (用于SSE)
                        // Remove possible "data: " prefix (for SSE)
                        let json_str = line.strip_prefix("data: ").unwrap_or(line);

                        serde_json::from_str::<serde_json::Value>(json_str)
                            .map_err(|err| {
                                Report::new(ChatError::ParseResponseError)
                                    .attach_printable(format!("Failed to parse JSON: {}", err))
                            })
                            .map(|json| {
                                // 提取内容
                                // Extract content
                                json.get("choices")
                                    .and_then(|c| c.as_array())
                                    .map(|choices| {
                                        choices
                                            .iter()
                                            .filter_map(|choice| choice.get("delta"))
                                            .filter_map(|delta| {
                                                delta.get("content").and_then(|c| c.as_str())
                                            })
                                            .for_each(|content| result.content.push_str(content));
                                    });

                                // 处理 usage 信息
                                // Process usage information
                                json.get("usage")
                                    .filter(|u| !u.is_null())
                                    .map(|usage| result.usage = Some(usage.clone()));
                            })
                    })?;

                Ok(result)
            })
            .await?;

        // 释放信号量许可
        // Release semaphore permit
        drop(semaphore_permit);
        Ok(result.content)
    }
}
