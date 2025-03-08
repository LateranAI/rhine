use bytes::Bytes;
use serde_json::json;

use error_stack::{Report, Result, ResultExt};
use thiserror::Error;

use futures::{Stream, TryStreamExt};
use tokio::sync::OwnedSemaphorePermit;
use reqwest::{Client, Error, Response};
use tracing::info;
use crate::chat::message::{Role, Session};

use crate::config::{Config, ModelCapability, THREAD_POOL};


#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Failed to assemble output description")]
    AssembleOutputDescriptionError,

    #[error("HTTP error with status code: {0}")]
    HttpError(u16),

    #[error("Timeout error")]
    TimeoutError,

    #[error("Failed to parse response")]
    ParseResponseError,

    #[error("Missing usage data")]
    MissingUsageData,

    #[error("Failed to get json")]
    GetJsonError,

    #[error("Failed to get function")]
    GetFunctionError,

    #[error("Operating on session failed")]
    SessionError,

    #[error("At least one character prompt required")]
    NoCharacterPrompts,

    #[error("Undefined character: {0}")]
    UndefinedCharacter(String),

    #[error("No character selected")]
    NoCharacterSelected,

    #[error("Unknown error")]
    UnknownError,
}

#[derive(Debug, Clone)]
pub struct BaseChat {
    pub model: String,

    pub base_url: String,

    pub api_key: String,

    pub client: Client,

    pub character_prompt: String,

    pub session: Session,

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
            session: Session::new(),
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
            session: Session::new(),
            usage: 0,
            need_stream,
        }
    }

    pub fn add_message_with_parent_path(
        &mut self,
        path: &[usize],
        role: Role,
        content: &str,
    ) -> Result<(), ChatError> {
        self.session
            .add_with_parent_path(path, role, content.to_string())
            .change_context(ChatError::SessionError)
    }

    pub fn add_message(&mut self, role: Role, content: &str) -> Result<(), ChatError> {
        self.session
            .add_with_default_path(role, content.to_string())
            .change_context(ChatError::SessionError)
    }

    pub fn build_request_body(
        &mut self,
        end_path: &[usize],
        current_speaker: &Role,
    ) -> Result<serde_json::Value, ChatError> {
        let messages_json = self
            .session
            .assemble_context(end_path, current_speaker)
            .change_context(ChatError::SessionError)?;

        Ok(json!({
            "model": self.model,
            "messages": messages_json,
            "stream": self.need_stream,
        }))
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
                let res = res.error_for_status().map_err(|e| {
                    Report::new(ChatError::HttpError(e.status().unwrap().as_u16()))
                        .attach_printable(format!("HTTP error with request body: {}", request_body))
                })?;

                let parsed: serde_json::Value = res
                    .json()
                    .await
                    .change_context(ChatError::ParseResponseError)
                    .attach_printable("Failed to parse response JSON")?;

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
        let semaphore_permit = THREAD_POOL
            .get(&self.base_url)
            .unwrap()
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let response = self.send_request(request_body.clone()).await;

        match response {
            Ok(res) => {
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
                        let json_str = line.strip_prefix("data: ").unwrap_or(line);

                        serde_json::from_str::<serde_json::Value>(json_str)
                            .map_err(|err| {
                                Report::new(ChatError::ParseResponseError)
                                    .attach_printable(format!("Failed to parse JSON: {}", err))
                            })
                            .map(|json| {
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
