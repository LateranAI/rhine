use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json::json;

use error_stack::{Report, Result, ResultExt};
use thiserror::Error;

use tracing::info;

use crate::chat::chat_base::{BaseChat, ChatError};
use crate::chat::chat_tool::ChatTool;
use crate::chat::message::Role;
use crate::config::ModelCapability;
use crate::prompt::assembler::assemble_output_description;
use crate::schema::json_schema::JsonSchema;

#[derive(Debug, Clone)]
pub struct MultiChat {
    pub base: BaseChat,

    character_prompts: HashMap<String, String>,

    pub current_character: String,

    need_stream: bool,
}

impl MultiChat {
    pub fn new_with_api_name(
        api_name: &str,
        character_prompts: HashMap<String, String>,
        need_stream: bool,
    ) -> Result<Self, ChatError> {
        if character_prompts.is_empty() {
            return Err(Report::new(ChatError::NoCharacterPrompts));
        }

        Ok(Self {
            base: BaseChat::new_with_api_name(api_name, "", need_stream),
            character_prompts,
            current_character: String::new(),
            need_stream,
        })
    }

    pub fn new_with_model_capability(
        model_capability: ModelCapability,
        character_prompts: HashMap<String, String>,
        need_stream: bool,
    ) -> Result<Self, ChatError> {
        if character_prompts.is_empty() {
            return Err(Report::new(ChatError::NoCharacterPrompts));
        }

        Ok(Self {
            base: BaseChat::new_with_model_capability(model_capability, "", need_stream),
            character_prompts,
            current_character: String::new(),
            need_stream,
        })
    }

    pub fn set_character(&mut self, character: &str) -> Result<(), ChatError> {
        if !self.character_prompts.contains_key(character) {
            return Err(Report::new(ChatError::UndefinedCharacter(
                character.to_owned(),
            )));
        }
        self.current_character = character.to_owned();
        self.base.character_prompt = self.character_prompts[&self.current_character].clone();
        Ok(())
    }

    pub fn add_user_message(&mut self, content: &str) -> Result<(), ChatError> {
        self.base.add_message(Role::User, content)
    }

    pub fn add_system_message(&mut self, content: &str) -> Result<(), ChatError> {
        self.base.add_message(Role::System, content)
    }

    pub fn add_message_with_parent_path(
        &mut self,
        path: &[usize],
        role: Role,
        content: &str,
    ) -> Result<(), ChatError> {
        self.base.add_message_with_parent_path(path, role, content)
    }

    pub async fn get_resp_with_new_question(
        &mut self,
        parent_path: &[usize],
        user_input: &str,
    ) -> Result<serde_json::Value, ChatError> {
        if self.current_character.is_empty() {
            return Err(Report::new(ChatError::NoCharacterSelected));
        }

        self.base
            .add_message_with_parent_path(parent_path, Role::User, user_input)?;

        let character_role = Role::Character(self.current_character.clone());

        Ok(self
            .base
            .build_request_body(&self.base.session.default_path.clone(), &character_role)?)
    }

    pub async fn get_resp_again(
        &mut self,
        end_path: &[usize],
    ) -> Result<serde_json::Value, ChatError> {
        if self.current_character.is_empty() {
            return Err(Report::new(ChatError::NoCharacterSelected));
        }

        let character_role = Role::Character(self.current_character.clone());

        Ok(self.base.build_request_body(end_path, &character_role)?)
    }

    pub async fn get_resp(&mut self, user_input: &str) -> Result<serde_json::Value, ChatError> {
        info!("path: {:?}", self.base.session.default_path.clone());
        self.get_resp_with_new_question(&self.base.session.default_path.clone(), user_input)
            .await
    }

    async fn get_content_from_resp(
        &mut self,
        request_body: serde_json::Value,
    ) -> Result<String, ChatError> {
        let content = if self.need_stream {
            let (stream, semaphore_permit) = self
                .base
                .get_stream_response(request_body.clone())
                .await
                .attach_printable("Failed to get stream response")?;

            BaseChat::get_content_from_stream_resp(stream, semaphore_permit)
                .await
                .attach_printable("Failed to extract content from stream response")?
        } else {
            let response = self
                .base
                .get_response(request_body.clone())
                .await
                .attach_printable("Failed to get response")?;

            BaseChat::get_content_from_resp(&response)
                .attach_printable("Failed to extract content from response")?
        };

        info!(
            "GetLLMAPIAnswer from {}: {}",
            self.current_character, content
        );

        let character_role = Role::Character(self.current_character.clone());
        self.base.add_message(character_role, &content)?;

        Ok(content)
    }

    pub async fn get_answer(&mut self, user_input: &str) -> Result<String, ChatError> {
        if self.current_character.is_empty() {
            return Err(Report::new(ChatError::NoCharacterSelected));
        }

        let request_body = self.get_resp(user_input).await?;

        self.get_content_from_resp(request_body).await
    }

    pub async fn get_json_answer<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
        user_input: &str,
    ) -> Result<T, ChatError> {
        let schema = T::json_schema();

        let output_description = assemble_output_description(schema.clone())
            .change_context(ChatError::AssembleOutputDescriptionError)
            .attach_printable(format!(
                "Failed to assemble output description for schema: {:?}",
                serde_json::to_string(&schema)
                    .unwrap_or_else(|_| "Schema serialization failed".to_string())
            ))?;

        self.base
            .add_message(Role::System, output_description.as_str())?;

        let answer = self.get_answer(user_input).await?;

        ChatTool::get_json::<T>(&answer, schema)
            .await
            .attach_printable(format!("Failed to parse answer as JSON: {}", answer))
    }

    pub async fn dialogue(
        &mut self,
        character: &str,
        user_input: &str,
    ) -> Result<String, ChatError> {
        self.set_character(character)?;
        self.add_user_message(user_input)?;
        self.get_answer(user_input).await
    }

    pub async fn structured_dialogue<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
        character: &str,
        user_input: &str,
    ) -> Result<T, ChatError> {
        self.set_character(character)?;
        self.add_user_message(user_input)?;
        self.get_json_answer::<T>(user_input).await
    }
}
