use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Invalid path")]
    InvalidPath,

    #[error("Invalid index: {0} at path: {1:?}")]
    InvalidIndex(usize, Vec<usize>),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

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
            other => Self::Character(other.to_string()), // 自定义角色转换 / Custom role conversion
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::System => "system".to_string(),
            Self::User => "user".to_string(),
            Self::Assistant => "assistant".to_string(),
            Self::Character(name) => name.clone(),
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Messages {
    pub role: Role,
    pub content: String,
    pub child: Vec<Messages>,
}

impl Messages {
    pub fn new(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            child: Vec::new(),
        }
    }

    pub fn get_node_by_path(&mut self, path: &[usize]) -> Result<&mut Messages, MessageError> {
        if path.is_empty() {
            return Ok(self);
        }

        if path[0] >= self.child.len() {
            return Err(MessageError::InvalidPath);
        }

        self.child[path[0]].get_node_by_path(&path[1..])
    }

    pub fn add_with_parent_path(
        &mut self,
        parent_path: &[usize],
        role: Role,
        content: String,
    ) -> Result<Vec<usize>, MessageError> {
        let parent = self.get_node_by_path(parent_path)?;
        let new_message = Self::new(role, content);
        parent.child.push(new_message);
        let mut new_default_path = parent_path.to_vec();
        new_default_path.push(parent.child.len() - 1);
        Ok(new_default_path)
    }

    pub fn to_api_format(&self, current_speaker: &Role) -> HashMap<String, String> {
        // 根据角色和当前发言者确定 API 格式
        // Determine API format based on role and current speaker
        let (role_str, content) = match &self.role {
            Role::System => ("system", self.content.clone()),
            Role::User => ("user", self.content.clone()),
            Role::Assistant => ("assistant", self.content.clone()),
            Role::Character(c) => {
                // 判断是否是当前发言者
                // Check if it's the current speaker
                if self.role == *current_speaker {
                    // 是发言者：作为 assistant 输出
                    // Is the speaker: output as assistant
                    ("assistant", self.content.clone())
                } else {
                    // 非发言者：添加前缀并作为 user 输出
                    // Not the speaker: add prefix and output as user
                    let prefixed_content = format!("{} said: {}", c, self.content);
                    ("user", prefixed_content)
                }
            }
        };

        // 创建并返回 API 格式的消息
        // Create and return message in API format
        HashMap::from([
            ("role".to_string(), role_str.to_string()),
            ("content".to_string(), content),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub message_roots: Vec<Messages>,
    pub default_path: Vec<usize>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            message_roots: Vec::new(),
            default_path: Vec::new(),
        }
    }

    pub fn get_node_by_path(&mut self, path: &[usize]) -> Result<&mut Messages, MessageError> {
        if path.is_empty() {
            return Err(MessageError::InvalidPath);
        }
        if path.len() == 1 {
            Ok(&mut self.message_roots[path[0]])
        } else {
            Ok(self.message_roots[path[0]].get_node_by_path(&path[1..])?)
        }
    }

    pub fn add_with_parent_path(
        &mut self,
        path: &[usize],
        role: Role,
        content: String,
    ) -> Result<(), MessageError> {
        if path.is_empty() {
            self.message_roots.push(Messages::new(role, content));
            self.default_path = vec![self.message_roots.len() - 1];
        } else {
            let mut new_default_path = vec![path[0]];
            new_default_path.append(&mut self.message_roots[path[0]].add_with_parent_path(&path[1..], role, content)?);
            self.default_path = new_default_path;
        }
        Ok(())
    }

    pub fn add_with_default_path(
        &mut self,
        role: Role,
        content: String,
    ) -> Result<(), MessageError> {
        self.add_with_parent_path(&self.default_path.clone(), role, content)
    }

    pub fn assemble_context(
        &mut self,
        end_path: &[usize],
        current_speaker: &Role,
    ) -> Result<Vec<HashMap<String, String>>, MessageError> {
        let mut node = self.get_node_by_path([end_path[0]].as_ref())?;
        let mut messages_vec = vec![node.to_api_format(current_speaker)];
        info!("node: {:?}", node);

        // 将for_each改为传统for循环
        for &idx in end_path[1..].iter() {
            node = &mut node.child[idx];
            messages_vec.push(node.to_api_format(current_speaker));
        }

        Ok(messages_vec)
    }
}
