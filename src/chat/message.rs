use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 聊天角色枚举
/// Chat role enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// 系统角色
    /// System role
    System,
    /// 用户角色
    /// User role
    User,
    /// 助手角色
    /// Assistant role
    Assistant,
    /// 自定义角色
    /// Custom character role
    #[serde(untagged)]
    Character(String),
}

impl From<&str> for Role {
    /// 从字符串创建角色
    ///
    /// Create role from string
    ///
    /// # 参数 / Parameters
    /// * `s` - 角色字符串 / Role string
    ///
    /// # 返回 / Returns
    /// * `Self` - 角色枚举 / Role enum
    fn from(s: &str) -> Self {
        match s {
            "system" => Self::System,
            "user" => Self::User,
            "assistant" => Self::Assistant,
            other => Self::Character(other.to_string()), // 自定义角色转换 / Custom role conversion
        }
    }
}

/// 消息结构体
/// Message structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Messages {
    /// 路径，表示消息在树中的位置
    /// Path, representing the position of the message in the tree
    pub path: Vec<usize>,

    /// 消息角色
    /// Message role
    pub role: Role,

    /// 消息内容
    /// Message content
    pub content: String,

    /// 子消息列表
    /// Child messages list
    pub child: Vec<Messages>,
}

impl Messages {
    /// 创建新的消息
    ///
    /// Create a new message
    ///
    /// # 参数 / Parameters
    /// * `role` - 消息角色 / Message role
    /// * `content` - 消息内容 / Message content
    ///
    /// # 返回 / Returns
    /// * `Self` - 新创建的消息 / Newly created message
    pub fn new(role: Role, content: String) -> Self {
        let path = Vec::new();
        let child = Vec::new();
        Self {
            path,
            role,
            content,
            child,
        }
    }

    /// 通过路径获取节点引用
    ///
    /// Get node reference by path
    ///
    /// # 参数 / Parameters
    /// * `path` - 节点路径 / Node path
    ///
    /// # 返回 / Returns
    /// * `Option<&Messages>` - 节点引用，如果路径无效则返回None / Node reference, returns None if path is invalid
    pub fn get_node_by_path(&self, path: &[usize]) -> Option<&Messages> {
        if path.is_empty() {
            return Some(self);
        }

        if path[0] >= self.child.len() {
            return None;
        }

        self.child[path[0]].get_node_by_path(&path[1..])
    }

    /// 通过路径获取可变节点引用
    ///
    /// Get mutable node reference by path
    ///
    /// # 参数 / Parameters
    /// * `path` - 节点路径 / Node path
    ///
    /// # 返回 / Returns
    /// * `Option<&mut Messages>` - 可变节点引用，如果路径无效则返回None / Mutable node reference, returns None if path is invalid
    pub fn get_node_by_path_mut(&mut self, path: &[usize]) -> Option<&mut Messages> {
        if path.is_empty() {
            return Some(self);
        }

        if path[0] >= self.child.len() {
            return None;
        }

        self.child[path[0]].get_node_by_path_mut(&path[1..])
    }

    /// 获取指定路径的终端子节点
    ///
    /// Get the terminal child node at the specified path
    ///
    /// # 参数 / Parameters
    /// * `path_to_end` - 终端节点路径 / Path to the terminal node
    ///
    /// # 返回 / Returns
    /// * `Option<&Messages>` - 终端节点，如果路径无效则返回None / Terminal node, returns None if path is invalid
    pub fn get_end_child(&self, path_to_end: &[usize]) -> Option<&Messages> {
        self.get_node_by_path(path_to_end)
    }

    /// 获取从自身开始到指定路径的所有子节点
    ///
    /// Get all child nodes from self to the specified path
    ///
    /// # 参数 / Parameters
    /// # 参数 / Parameters
    /// * `path_to_end` - 终端节点路径 / Path to the terminal node
    ///
    /// # 返回 / Returns
    /// * `Vec<Messages>` - 子节点列表 / List of child nodes
    pub fn get_children_from_self(&self, path_to_end: &[usize]) -> Vec<Messages> {
        if path_to_end.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();

        if path_to_end[0] < self.child.len() {
            // 添加当前层级的节点
            // Add node at current level
            result.push(self.child[path_to_end[0]].clone());

            // 递归获取下一层级的节点
            // Recursively get nodes at next level
            if path_to_end.len() > 1 {
                let mut children = self.child[path_to_end[0]].get_children_from_self(&path_to_end[1..]);
                result.append(&mut children);
            }
        }

        result
    }

    /// 获取从起始路径到终端路径的所有子节点
    ///
    /// Get all child nodes from start path to end path
    ///
    /// # 参数 / Parameters
    /// * `path_to_start` - 起始节点路径 / Path to the start node
    /// * `path_to_end` - 终端节点路径 / Path to the end node
    ///
    /// # 返回 / Returns
    /// * `Vec<Messages>` - 子节点列表 / List of child nodes
    pub fn get_children_from_start(&self, path_to_start: &[usize], path_to_end: &[usize]) -> Vec<Messages> {
        if let Some(start_node) = self.get_node_by_path(path_to_start) {
            start_node.get_children_from_self(path_to_end)
        } else {
            Vec::new()
        }
    }

    /// 获取从根节点到指定路径的所有节点
    ///
    /// Get all nodes from root to the specified path
    ///
    /// # 参数 / Parameters
    /// * `path_to_end` - 终端节点路径 / Path to the terminal node
    ///
    /// # 返回 / Returns
    /// * `Vec<Messages>` - 节点列表 / List of nodes
    pub fn get_children_from_root(&self, path_to_end: &[usize]) -> Vec<Messages> {
        let mut result = Vec::new();

        // 添加根节点
        // Add root node
        result.push(self.clone());

        // 逐步构建路径并添加每个节点
        // Build path step by step and add each node
        let mut current_path = Vec::new();

        for &idx in path_to_end {
            if idx >= self.child.len() {
                break;
            }

            current_path.push(idx);
            if let Some(node) = self.get_node_by_path(&current_path) {
                result.push(node.clone());
            } else {
                break;
            }
        }

        result
    }

    /// 在指定路径添加新消息
    ///
    /// Add a new message at the specified path
    ///
    /// # 参数 / Parameters
    /// * `path` - 父节点路径 / Parent node path
    /// * `role` - 消息角色 / Message role
    /// * `content` - 消息内容 / Message content
    ///
    /// # 返回 / Returns
    /// * `Result<(), String>` - 成功返回Ok，失败返回错误信息 / Returns Ok on success, error message on failure
    pub fn add(&mut self, path: &[usize], role: Role, content: String) -> Result<(), String> {
        let parent = self.get_node_by_path_mut(path)
            .ok_or_else(|| format!("Invalid path: {:?}", path))?;

        // 创建新消息并设置路径
        // Create new message and set path
        let mut new_message = Messages::new(role, content);
        let mut new_path = path.to_vec();
        new_path.push(parent.child.len());
        new_message.path = new_path;

        // 添加到父节点的子列表
        // Add to parent's child list
        parent.child.push(new_message);

        Ok(())
    }

    /// 更新指定路径的消息内容
    ///
    /// Update message content at the specified path
    ///
    /// # 参数 / Parameters
    /// * `path` - 节点路径 / Node path
    /// * `content` - 新的消息内容 / New message content
    ///
    /// # 返回 / Returns
    /// * `Result<(), String>` - 成功返回Ok，失败返回错误信息 / Returns Ok on success, error message on failure
    pub fn update_content(&mut self, path: &[usize], content: String) -> Result<(), String> {
        let node = self.get_node_by_path_mut(path)
            .ok_or_else(|| format!("Invalid path: {:?}", path))?;

        node.content = content;

        Ok(())
    }

    /// 删除指定路径的消息及其所有子消息
    ///
    /// Delete the message at the specified path and all its child messages
    ///
    /// # 参数 / Parameters
    /// * `path` - 节点路径 / Node path
    ///
    /// # 返回 / Returns
    /// * `Result<(), String>` - 成功返回Ok，失败返回错误信息 / Returns Ok on success, error message on failure
    pub fn delete(&mut self, path: &[usize]) -> Result<(), String> {
        if path.is_empty() {
            return Err("Cannot delete root node".to_string());
        }

        let parent_path = &path[0..path.len()-1];
        let index = path[path.len() - 1];

        let parent = self.get_node_by_path_mut(parent_path)
            .ok_or_else(|| format!("Invalid parent path: {:?}", parent_path))?;

        if index >= parent.child.len() {
            return Err(format!("Invalid index: {}", index));
        }

        // 删除节点
        // Delete node
        parent.child.remove(index);

        // 更新剩余子节点的路径
        // Update paths of remaining child nodes
        for (i, child) in parent.child.iter_mut().enumerate().skip(index) {
            Self::update_node_paths(child, parent_path, i);
        }

        Ok(())
    }

    /// 更新节点及其所有子节点的路径
    ///
    /// Update the paths of a node and all its child nodes
    ///
    /// # 参数 / Parameters
    /// * `node` - 要更新的节点 / Node to update
    /// * `parent_path` - 父节点路径 / Parent node path
    /// * `index` - 节点在父节点子列表中的索引 / Index of the node in parent's child list
    fn update_node_paths(node: &mut Messages, parent_path: &[usize], index: usize) {
        // 更新当前节点路径
        // Update current node path
        node.path = parent_path.to_vec();
        node.path.push(index);

        // 递归更新所有子节点路径
        // Recursively update all child node paths
        for (i, child) in node.child.iter_mut().enumerate() {
            Self::update_node_paths(child, &node.path, i);
        }
    }

    /// 将消息转换为 API 格式
    ///
    /// Convert message to API format
    ///
    /// # 参数 / Parameters
    /// * `current_speaker` - 当前发言者角色 / Current speaker role
    ///
    /// # 返回 / Returns
    /// * `HashMap<String, String>` - API 格式的消息 / Message in API format
    pub fn to_api_format_single(&self, current_speaker: &Role) -> HashMap<String, String> {
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

        HashMap::from([
            ("role".to_string(), role_str.to_string()),
            ("content".to_string(), content),
        ])
    }

    /// 获取指定路径之间的对话历史（用于API调用）
    ///
    /// Get conversation history between specified paths (for API calls)
    ///
    /// # 参数 / Parameters
    /// * `start_path` - 起始节点路径 / Path to the start node
    /// * `end_path` - 终端节点路径 / Path to the end node
    /// * `current_speaker` - 当前发言者角色 / Current speaker role
    ///
    /// # 返回 / Returns
    /// * `Vec<HashMap<String, String>>` - API格式的对话历史 / Conversation history in API format
    pub fn assemble_context(
        &self,
        start_path: &[usize],
        end_path: &[usize],
        current_speaker: &Role
    ) -> Vec<HashMap<String, String>> {
        let mut history = Vec::new();

        // 找到最近的共同祖节点
        // Find the nearest common ancestor
        let common_prefix_len = start_path.iter()
            .zip(end_path.iter())
            .take_while(|&(&a, &b)| a == b)
            .count();

        let common_ancestor_path = &start_path[0..common_prefix_len];

        // 构建从共同祖先到终端节点的路径
        // Build path from common ancestor to end node
        let mut nodes = Vec::new();

        if let Some(ancestor) = self.get_node_by_path(common_ancestor_path) {
            // 添加从祖先到起始节点的路径（若需要）
            // Add path from ancestor to start node (if needed)
            if common_prefix_len < start_path.len() {
                let mut current_path = common_ancestor_path.to_vec();
                for &idx in &start_path[common_prefix_len..] {
                    current_path.push(idx);
                    if let Some(node) = self.get_node_by_path(&current_path) {
                        nodes.push(node);
                    }
                }
            } else {
                // 起始节点就是共同祖先
                // Start node is the common ancestor
                nodes.push(ancestor);
            }

            // 添加从祖先到终端节点的路径
            // Add path from ancestor to end node
            if common_prefix_len < end_path.len() {
                let mut current_path = common_ancestor_path.to_vec();
                for &idx in &end_path[common_prefix_len..] {
                    current_path.push(idx);
                    if let Some(node) = self.get_node_by_path(&current_path) {
                        if !nodes.contains(&node) {
                            nodes.push(node);
                        }
                    }
                }
            }
        }

        // 转换为API格式
        // Convert to API format
        for node in nodes {
            history.push(node.to_api_format_single(current_speaker));
        }

        history
    }
}
