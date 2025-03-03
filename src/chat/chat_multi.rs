// 标准库引用 / Standard library imports
use std::collections::HashMap;

// 外部库引用 / External library imports (按泛用程度从高到低排序 / ordered by generality from high to low)
// 基础数据类型和序列化 / Basic data types and serialization
use serde::de::DeserializeOwned;

// 错误处理 / Error handling
use error_stack::{Report, Result, ResultExt};

// 日志记录 / Logging
use tracing::log::info;

// 本地库引用 / Local library imports
use crate::chat::chat_base::{BaseChat, ChatError, Role};
use crate::chat::chat_tool::ChatTool;
use crate::config::ModelCapability;
use crate::prompt::assembler::assemble_output_description;
use crate::schema::json_schema::JsonSchema;


/// 多角色聊天错误枚举
/// Multi-character chat error enumeration
// #[derive(Debug, Error)]
// pub enum ChatError {
//     /// 没有提供角色提示词
//     /// No character prompts provided
//     #[error("At least one character prompt required")]
//     NoCharacterPrompts,
//     
//     /// 未定义的角色
//     /// Undefined character
//     #[error("Undefined character: {0}")]
//     UndefinedCharacter(String),
//     
//     /// 未选择角色
//     /// No character selected
//     #[error("No character selected")]
//     NoCharacterSelected,
//     
//     /// 组装输出描述失败
//     /// Failed to assemble output description
//     #[error("Failed to assemble output description")]
//     AssembleOutputDescription,
//     
//     /// API 请求失败
//     /// API request failed
//     #[error("API request failed: {0}")]
//     ApiRequestFailed(String),
//     
//     /// 解析响应失败
//     /// Failed to parse response
//     #[error("Failed to parse response")]
//     ParseResponseFailed,
//     
//     /// 流式响应错误
//     /// Streaming response error
//     #[error("Streaming response error: {0}")]
//     StreamingError(String),
// }

/// 将 ChatError 转换为 ChatError
/// Convert ChatError to ChatError
// impl From<ChatError> for ChatError {
//     fn from(err: ChatError) -> Self {
//         ChatError::ApiRequestFailed(format!("{:?}", err))
//     }
// }

/// 多角色聊天结构体，支持在多个预定义角色间切换
/// Multi-character chat structure, supports switching between predefined characters
#[derive(Debug, Clone)]
pub struct MultiChat {
    /// 基础聊天实例
    /// Base chat instance
    pub base: BaseChat,
    
    /// 角色提示词映射
    /// Character prompts mapping
    character_prompts: HashMap<String, String>,
    
    /// 当前选择的角色
    /// Currently selected character
    pub current_character: String,
    
    /// 是否需要流式响应
    /// Whether streaming response is needed
    need_stream: bool,
}

impl MultiChat {
    /// 创建新的多角色聊天实例
    /// 
    /// Create a new multi-character chat instance
    ///
    /// # 参数 / Parameters
    /// * `api_name` - API 名称 / API name
    /// * `character_prompts` - 角色提示词映射 / Character prompts mapping
    /// * `need_stream` - 是否需要流式响应 / Whether streaming response is needed
    ///
    /// # 返回 / Returns
    /// * `Result<Self, ChatError>` - 多角色聊天实例或错误 / Multi-character chat instance or error
    pub fn new(
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

    /// 使用模型能力创建新的多角色聊天实例
    /// 
    /// Create a new multi-character chat instance with model capability
    ///
    /// # 参数 / Parameters
    /// * `model_capability` - 模型能力枚举 / Model capability enum
    /// * `character_prompts` - 角色提示词映射 / Character prompts mapping
    /// * `need_stream` - 是否需要流式响应 / Whether streaming response is needed
    ///
    /// # 返回 / Returns
    /// * `Result<Self, ChatError>` - 多角色聊天实例或错误 / Multi-character chat instance or error
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
    
    /// 设置当前角色
    /// 
    /// Set current character
    ///
    /// # 参数 / Parameters
    /// * `character` - 角色名称 / Character name
    ///
    /// # 返回 / Returns
    /// * `Result<(), ChatError>` - 成功或错误 / Success or error
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

    /// 获取用户输入的回答
    /// 
    /// Get answer for the current conversation state
    ///
    /// # 返回 / Returns
    /// * `Result<String, ChatError>` - 回答内容或错误 / Answer content or error
    pub async fn get_answer(&mut self) -> Result<String, ChatError> {
        if self.current_character.is_empty() {
            return Err(Report::new(ChatError::NoCharacterSelected));
        }

        let request_body = self.base.build_request_body();
        
        let content = if self.need_stream {
            // 使用流式响应
            // Use streaming response
            let (stream, semaphore_permit) = self
                .base
                .get_stream_response(request_body)
                .await?;
                
            BaseChat::get_content_from_stream_resp(stream, semaphore_permit).await?

        } else {
            // 使用普通响应
            // Use normal response
            let response = self
                .base
                .get_response(request_body)
                .await?;
                
            BaseChat::get_content_from_resp(&response)?
        };

        info!("MultiChat answer from {}: {}", self.current_character, content);
        
        // 添加角色消息
        // Add character message
        self.base.add_message(Role::Character(self.current_character.clone()), &content);

        Ok(content)
    }

    /// 添加用户消息
    /// 
    /// Add user message
    ///
    /// # 参数 / Parameters
    /// * `content` - 消息内容 / Message content
    pub fn add_user_message(&mut self, content: &str) {
        self.base.add_message(Role::User, content);
    }

    /// 添加系统消息
    /// 
    /// Add system message
    ///
    /// # 参数 / Parameters
    /// * `content` - 消息内容 / Message content
    pub fn add_system_message(&mut self, content: &str) {
        self.base.add_message(Role::System, content);
    }

    /// 获取用户输入的结构化 JSON 回答
    /// 
    /// Get structured JSON answer for the current conversation state
    ///
    /// # 类型参数 / Type Parameters
    /// * `T` - 目标结构体类型，需要实现 DeserializeOwned + JsonSchema / Target struct type, must implement DeserializeOwned + JsonSchema
    ///
    /// # 返回 / Returns
    /// * `Result<T, ChatError>` - 结构化数据或错误 / Structured data or error
    pub async fn get_json_answer<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
    ) -> Result<T, ChatError> {
        let schema = T::json_schema();
        
        // 添加输出描述系统消息
        // Add output description system message
        self.base.add_message(
            Role::System,
            assemble_output_description(schema.clone())
                .change_context(ChatError::AssembleOutputDescriptionError)?
                .as_str(),
        );
        
        let answer = self.get_answer().await?;
        info!("GetLLMAPIAnswer: {}", answer);

        // 解析 JSON 回答
        // Parse JSON answer
        ChatTool::get_json::<T>(&answer, schema).await
    }
    
    /// 在指定角色和用户之间进行对话
    /// 
    /// Conduct dialogue between specified character and user
    ///
    /// # 参数 / Parameters
    /// * `character` - 角色名称 / Character name
    /// * `user_input` - 用户输入 / User input
    ///
    /// # 返回 / Returns
    /// * `Result<String, ChatError>` - 角色回答或错误 / Character answer or error
    pub async fn dialogue(&mut self, character: &str, user_input: &str) -> Result<String, ChatError> {
        self.set_character(character)?;
        self.add_user_message(user_input);
        self.get_answer().await
    }
    
    /// 在指定角色和用户之间进行结构化对话
    /// 
    /// Conduct structured dialogue between specified character and user
    ///
    /// # 参数 / Parameters
    /// * `character` - 角色名称 / Character name
    /// * `user_input` - 用户输入 / User input
    ///
    /// # 返回 / Returns
    /// * `Result<T, ChatError>` - 结构化角色回答或错误 / Structured character answer or error
    pub async fn structured_dialogue<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
        character: &str,
        user_input: &str,
    ) -> Result<T, ChatError> {
        self.set_character(character)?;
        self.add_user_message(user_input);
        self.get_json_answer::<T>().await
    }
}