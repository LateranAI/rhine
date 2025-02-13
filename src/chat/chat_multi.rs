use std::collections::HashMap;
use error_stack::{Report, Result, ResultExt};
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::chat::chat_base::{BaseChat, Role};
use crate::chat::chat_tool::ChatTool;
use crate::prompt::assembler::{assemble_output_description};
use crate::schema::json_schema::JsonSchema;


#[derive(Debug, Error)]
pub enum MultiChatError {
    #[error("At least one character prompt required")]
    NoCharacterPrompts,
    #[error("Undefined character: {0}")]
    UndefinedCharacter(String),
    #[error("No character selected")]
    NoCharacterSelected,
    #[error("Failed to assemble output description")] // 新增的错误类型
    AssembleOutputDescription,
    #[error("API request failed")] // 新增
    ApiRequestFailed,
    #[error("Failed to parse response")] //新增
    ParseResponseFailed,
}

#[derive(Debug, Clone)]
pub struct MultiChat {
    pub base: BaseChat,
    character_prompts: HashMap<String, String>,
    pub current_character: String,
}

impl MultiChat {
    pub fn new(
        model: &str,
        base_url: &str,
        api_key: &str,
        character_prompts: HashMap<String, String>,
        need_stream: bool,
    ) -> Result<Self, MultiChatError> {
        if character_prompts.is_empty() {
            return Err(Report::new(MultiChatError::NoCharacterPrompts));
        }

        Ok(Self {
            base: BaseChat::new(model, base_url, api_key, "", need_stream),
            character_prompts,
            current_character: String::new(),
        })
    }

    pub fn set_character(&mut self, character: &str) -> Result<(), MultiChatError> {
        if !self.character_prompts.contains_key(character) {
            return Err(Report::new(MultiChatError::UndefinedCharacter(
                character.to_owned(),
            )));
        }
        self.current_character = character.to_owned();
        self.base.character_prompt = self.character_prompts[&self.current_character].clone();
        Ok(())
    }

    pub async fn get_answer(&mut self) -> Result<String, MultiChatError> {
        if self.current_character.is_empty() {
            return Err(Report::new(MultiChatError::NoCharacterSelected));
        }

        let request_body = self.base.build_request_body();

        let response = self
            .base
            .send_request(request_body)
            .await
            .change_context(MultiChatError::ApiRequestFailed)?; // 增加了错误处理

        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(MultiChatError::ParseResponseFailed)?; // 增加了错误处理

        self.base
            .add_message(Role::Character(self.current_character.clone()), content);

        Ok(content.to_string())
    }

    pub async fn get_json_answer<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
    ) -> Result<T, MultiChatError> {
        let schema = T::json_schema();
        self.base.add_message(
            Role::System,
            assemble_output_description(schema.clone())
                .change_context(MultiChatError::AssembleOutputDescription)?
                .as_str(),
        );
        let answer = self.get_answer().await?;

        ChatTool::get_json::<T>(answer.as_str(), schema)
            .await
            .change_context(MultiChatError::ParseResponseFailed) // 添加错误上下文
            .attach_printable(format!("Failed to parse JSON response: {}", answer))
    }
}
