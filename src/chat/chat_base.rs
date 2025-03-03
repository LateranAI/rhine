use crate::config::{CFG, Config, ModelCapability, THREAD_POOL};
use error_stack::{Context, Report, Result, ResultExt};
use reqwest::{Client, Error, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use futures::{Stream, TryStreamExt};
use thiserror::Error;
use tokio_stream::StreamExt;
use bytes::Bytes;
use tokio::sync::OwnedSemaphorePermit;

#[derive(Debug, Error)]
pub enum ChatError {
    // prompt
    #[error("Failed to assemble output description")]
    AssembleOutputDescriptionError,

    // Http connection
    #[error("HTTP error with status code: {0}")]
    HttpError(u16),
    #[error("Timeout error")]
    TimeoutError,

    // result
    #[error("Failed to parse response")]
    ParseResponseError,
    #[error("Missing usage data")]
    MissingUsageData,

    // tool use
    #[error("Failed to get json")]
    GetJsonError,
    #[error("Failed to get function")]
    GetFunctionError,
    #[error("Unknown error")]
    UnknownError,
}

// ---------- 基础聊天结构 ----------
#[derive(Debug, Clone)]
pub struct BaseChat {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub client: Client,
    pub character_prompt: String,
    pub messages: Vec<Message>,
    pub usage: i32,
    pub need_stream: bool,
}

impl BaseChat {
    pub fn new_with_api_name(api_name: &str, character_prompt: &str, need_stream: bool) -> Self {
        let api_info = Config::get_api_info_with_name(api_name.to_string()).unwrap();

        Self {
            model: api_info.model,
            base_url: api_info.base_url,
            api_key: api_info.api_key,
            client: api_info.client,
            character_prompt: character_prompt.to_string(),
            messages: Vec::new(),
            usage: 0,
            need_stream,
        }
    }

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
            messages: Vec::new(),
            usage: 0,
            need_stream,
        }
    }

    pub fn add_message(&mut self, role: Role, content: &str) {
        self.messages.push(Message {
            role,
            content: content.to_string(),
        });
    }

    pub fn build_messages(&self) -> Vec<HashMap<String, String>> {
        let mut messages = vec![HashMap::from([
            ("role".to_owned(), "system".to_owned()),
            ("content".to_owned(), self.character_prompt.clone()),
        ])];

        messages.extend(
            self.messages
                .iter()
                .map(|m| m.to_api_format(&m.role))
                .collect::<Vec<_>>(),
        );

        messages
    }

    pub fn build_request_body(&self) -> serde_json::Value {
        let messages = self.build_messages();

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": self.need_stream,
        });

        body
    }

    pub async fn send_request(
        &mut self,
        request_body: serde_json::Value,
    ) -> core::result::Result<Response, Error> {
        self.client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .bearer_auth(&self.api_key)
            .json(&request_body)
            // .timeout(Duration::from_secs(5))
            .send()
            .await
    }

    pub async fn get_response(
        &mut self,
        request_body: serde_json::Value,
    ) -> Result<serde_json::Value, ChatError> {
        let semaphore_permit = THREAD_POOL
            .get(&self.base_url)
            .unwrap()
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let response = self.send_request(request_body.clone()).await;

        drop(semaphore_permit);

        match response {
            Ok(res) => {
                // 处理 HTTP 状态码错误
                let res = res.error_for_status().map_err(|e| {
                    Report::new(ChatError::HttpError(e.status().unwrap().as_u16()))
                        .attach_printable(format!("HTTP error with request body: {}", request_body))
                })?;

                // 解析 JSON 响应
                let parsed: serde_json::Value = res
                    .json()
                    .await
                    .change_context(ChatError::ParseResponseError)
                    .attach_printable("Failed to parse response JSON")?;

                // 更新 token 使用量
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

    pub fn get_content_from_resp(resp: &serde_json::Value) -> Result<String, ChatError> {
        let content = resp.get("choices")
            .and_then(|c| {c.get(0)})
            .and_then(|c| {c.get("message")})
            .and_then(|m| {m.get("content")});
        match content {
            Some(content) => Ok(content.to_string()),
            None => {
                 Err(Report::new(ChatError::ParseResponseError))
                    .attach_printable("Failed to parse response content")
            }
        }
    }

    pub async fn get_stream_response(
        &mut self,
        request_body: serde_json::Value,
    ) -> Result<(impl Stream<Item=reqwest::Result<Bytes>>  + Send + Unpin, OwnedSemaphorePermit), ChatError> {
        let semaphore_permit = THREAD_POOL
            .get(&self.base_url)
            .unwrap()
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let response = self.send_request(request_body.clone()).await;

        // drop(semaphore_permit);

        match response {
            Ok(res) => {
                // 处理 HTTP 状态码错误
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

    pub async fn get_content_from_stream_resp(
        stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + Unpin,
        semaphore_permit: OwnedSemaphorePermit,
    ) -> Result<String, ChatError> {
        // 创建用于收集结果的结构
        #[derive(Default)]
        struct StreamResult {
            content: String,
            usage: Option<serde_json::Value>,
        }

        let result = stream
            .map_err(|err| Report::new(ChatError::HttpError(0))
                .attach_printable(format!("Failed to get response: {}", err)))
            .try_fold(StreamResult::default(), |mut result, chunk| async move {
                String::from_utf8_lossy(&chunk)
                    .split('\n')
                    .filter(|line| !line.is_empty() && *line != "data: [DONE]")
                    .try_for_each(|line| {
                        // 移除可能的 "data: " 前缀 (用于SSE)
                        let json_str = line.strip_prefix("data: ").unwrap_or(line);

                        serde_json::from_str::<serde_json::Value>(json_str)
                            .map_err(|err| Report::new(ChatError::ParseResponseError)
                                .attach_printable(format!("Failed to parse JSON: {}", err)))
                            .map(|json| {
                                json.get("choices")
                                    .and_then(|c| c.as_array())
                                    .map(|choices| {
                                        choices.iter()
                                            .filter_map(|choice| choice.get("delta"))
                                            .filter_map(|delta| delta.get("content").and_then(|c| c.as_str()))
                                            .for_each(|content| result.content.push_str(content));
                                    });
                                // 处理usage信息
                                json.get("usage")
                                    .filter(|u| !u.is_null())
                                    .map(|usage| result.usage = Some(usage.clone()));
                            })
                    })?;

                Ok(result)
            })
            .await?;

        drop(semaphore_permit);
        Ok(result.content)
    }
}

// ---------- 数据结构 ----------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    #[serde(untagged)]
    Character(String),
}

impl From<&str> for Role {
    fn from(s: &str) -> Self {
        match s {
            "system" => Self::System,
            "user" => Self::User,
            "assistant" => Self::Assistant,
            other => Self::Character(other.to_string()), // 关键转换！
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn to_api_format(&self, current_speaker: &Role) -> HashMap<String, String> {
        let (mut role_str, mut content) = match &self.role {
            Role::System => ("system", self.content.clone()),
            Role::User => ("user", self.content.clone()),
            Role::Assistant => ("assistant", self.content.clone()),
            Role::Character(c) => {
                // 判断是否是当前发言者
                if self.role == *current_speaker {
                    // 是发言者：作为assistant输出
                    ("assistant", self.content.clone())
                } else {
                    // 非发言者：添加前缀并作为user输出
                    let prefixed_content = format!("{} said: {}", c, self.content);
                    ("user", prefixed_content)
                }
            }
        };

        // 针对Assistant角色的特殊处理（可选）
        if let Role::Assistant = self.role {
            if self.role == *current_speaker {
                role_str = "assistant";
            } else {
                role_str = "user";
                content = format!("Assistant said: {}", self.content);
            }
        }

        HashMap::from([
            ("role".to_string(), role_str.to_string()),
            ("content".to_string(), content),
        ])
    }
}
