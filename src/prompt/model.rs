// 标准库
use std::collections::HashMap;

// 序列化/反序列化
use serde::Deserialize;

// 错误处理
use error_stack::{Report, Result, ResultExt};
use thiserror::Error;

// 项目内部模块
use crate::prompt::assembler::assemble;
use crate::prompt::loader::load;

/// 提示模型错误枚举
/// Prompt model error enum
#[derive(Debug, Error)]
pub enum PromptModelError {
    /// 加载提示失败
    /// Failed to load prompts
    #[error("Failed to load prompts")]
    LoadError,
    
    /// 提示初始化失败
    /// Failed to initialize prompts
    #[error("Failed to initialize prompts")]
    InitError,
    
    /// 角色提示不存在
    /// Character prompt does not exist
    #[error("Character prompt not found: {0}")]
    CharacterPromptNotFound(String),
    
    /// 阶段提示不存在
    /// Stage prompt does not exist
    #[error("Stage prompt not found: {0}")]
    StagePromptNotFound(String),
}

//======================================================================
// 配置文件结构定义
// Configuration file structure definitions
//======================================================================

/// 配置结构体，定义模板路径和提示信息
/// Configuration struct defining template path and prompt information
#[derive(Debug, Deserialize)]
pub struct Config {
    /// 模板文件路径
    /// Template file path
    pub template_path: String,
    
    /// 提示信息列表
    /// List of prompt information
    pub prompt_info: Vec<Info>,
}

/// 提示信息结构体，包含名称、描述和路径
/// Prompt information struct containing name, description and path
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
pub struct Info {
    /// 提示名称
    /// Prompt name
    pub name: String,
    
    /// 提示描述
    /// Prompt description
    pub description: String,
    
    /// 提示文件路径
    /// Prompt file path
    pub path: String,
}

//======================================================================
// TOML模板结构定义
// TOML template structure definitions
//======================================================================

/// 模板结构体，包含角色提示模板
/// Template struct containing character prompt templates
#[derive(Debug, Deserialize)]
pub struct Template {
    /// 角色提示模板
    /// Character prompt templates
    pub character_prompts: CharacterPromptsTemplate,
}

/// 角色提示模板结构体，定义各种提示元素
/// Character prompts template struct defining various prompt elements
#[derive(Debug, Deserialize)]
pub struct CharacterPromptsTemplate {
    /// 任务描述模板元素
    /// Task description template element
    pub task_description: TemplateElement,
    
    /// 阶段描述模板元素
    /// Stage description template element
    pub stage_description: TemplateElement,
    
    /// 输入描述模板元素
    /// Input description template element
    pub input_description: TemplateElement,
    
    /// 输出描述模板元素
    /// Output description template element
    pub output_description: TemplateElement,
    
    /// 原则模板元素
    /// Principle template element
    pub principle: TemplateElement,
    
    /// 思考方式模板元素
    /// How to think template element
    pub how_to_think: TemplateElement,
    
    /// 示例模板元素
    /// Examples template element
    pub examples: TemplateElement,
}

/// 模板元素结构体，包含元素名称和描述
/// Template element struct containing element name and description
#[derive(Debug, Deserialize, Default)]
pub struct TemplateElement {
    /// 元素名称
    /// Element name
    pub element_name: String,
    
    /// 元素描述
    /// Element description
    pub description: String,
}

//======================================================================
// TOML内容结构定义
// TOML content structure definitions
//======================================================================

/// 内容结构体，包含角色提示和阶段提示
/// Content struct containing character prompts and stage prompts
#[derive(Clone, Debug, Deserialize, Default)]
pub struct Content {
    /// 角色提示
    /// Character prompts
    pub character_prompts: CharacterPrompts,
    
    /// 阶段提示列表，默认为空
    /// Stage prompt list, defaults to empty
    #[serde(default)]
    pub stage_prompt: Vec<StagePrompt>
}

/// 返回默认角色名称列表
/// Returns default character names list
fn default_character_names() -> Vec<String> {
    vec!["assistant".to_string()]
}

/// 角色提示结构体，包含角色名称和各种提示映射
/// Character prompts struct containing character names and various prompt mappings
#[derive(Clone, Debug, Deserialize, Default)]
pub struct CharacterPrompts {
    /// 角色名称列表，默认为["assistant"]
    /// Character names list, defaults to ["assistant"]
    #[serde(default = "default_character_names")]
    pub character_names: Vec<String>,
    
    /// 任务描述映射，默认为空
    /// Task description mapping, defaults to empty
    #[serde(default)]
    pub task_description: HashMap<String, String>,
    
    // 暂时注释掉的字段
    // Temporarily commented fields
    // #[serde(default)]
    // pub input_description: HashMap<String, String>,
    // #[serde(default)]
    // pub output_description: HashMap<String, String>,
    
    /// 原则映射，默认为空
    /// Principle mapping, defaults to empty
    #[serde(default)]
    pub principle: HashMap<String, String>,
    
    /// 思考方式映射，默认为空
    /// How to think mapping, defaults to empty
    #[serde(default)]
    pub how_to_think: HashMap<String, String>,
    
    /// 示例映射，默认为空
    /// Examples mapping, defaults to empty
    #[serde(default)]
    pub examples: HashMap<String, String>,
}

/// 阶段提示结构体，包含名称、描述和内容
/// Stage prompt struct containing name, description and content
#[derive(Clone, Debug, Deserialize, Default)]
pub struct StagePrompt {
    /// 阶段名称
    /// Stage name
    pub name: String,
    
    /// 阶段描述
    /// Stage description
    pub description: String,
    
    /// 阶段内容
    /// Stage content
    pub content: String,
}

//======================================================================
// 提示词结构体定义
// Prompt struct definitions
//======================================================================

/// 提示词集合结构体，包含所有提示
/// Prompts collection struct containing all prompts
#[derive(Clone, Debug)]
pub struct Prompts {
    /// 信息与内容的映射
    /// Mapping between information and content
    pub info_with_contents: HashMap<Info, Content>,
    
    /// 获取搜索关键词的提示
    /// Get search keywords prompt
    pub get_search_keywords: Prompt,
    
    /// 获取论文评分的提示
    /// Get paper scores prompt
    pub get_paper_scores: Prompt,
    
    /// 获取论文概览的提示
    /// Get paper overview prompt
    pub get_paper_overview: Prompt,
    
    /// 获取带评论的笔记的提示
    /// Get note with review prompt
    pub get_note_with_review: Prompt,
    
    /// 讨论论文细节的提示
    /// Discuss paper details prompt
    pub discuss_paper_details: Prompt,
    
    /// 获取带讨论的笔记的提示
    /// Get note with discussion prompt
    pub get_note_with_discussion: Prompt,
}

impl Prompts {
    /// 初始化提示词集合
    /// Initialize prompts collection
    ///
    /// # 返回 (Returns)
    /// * `Result<Self, PromptModelError>` - 成功返回初始化的提示词集合，失败返回错误
    ///                                    - Returns initialized prompts collection on success, error on failure
    pub fn init() -> Result<Self, PromptModelError> {
        // 加载模板和内容
        // Load template and content
        let (template, info_with_contents) = load()
            .change_context(PromptModelError::LoadError)?;
        
        // 组装提示词
        // Assemble prompts
        let filename_with_prompts = assemble(&template, &info_with_contents);
        
        // 从映射中提取各个提示词，添加错误处理
        // Extract each prompt from the mapping, add error handling
        let get_prompt = |name: &str| -> Result<Prompt, PromptModelError> {
            filename_with_prompts.get(name)
                .cloned()
                .ok_or_else(|| Report::new(PromptModelError::InitError)
                    .attach_printable(format!("Prompt not found: {}", name)))
        };
        
        Ok(Self {
            info_with_contents,
            get_search_keywords: get_prompt("get_search_keywords")?,
            get_paper_scores: get_prompt("get_paper_scores")?,
            get_paper_overview: get_prompt("get_paper_overview")?,
            get_note_with_review: get_prompt("get_note_with_review")?,
            discuss_paper_details: get_prompt("discuss_paper_details")?,
            get_note_with_discussion: get_prompt("get_note_with_discussion")?,
        })
    }
    
    /// 初始化提示词集合（无错误处理版本，保持向后兼容）
    /// Initialize prompts collection (no error handling version, for backward compatibility)
    ///
    /// # 返回 (Returns)
    /// * `Self` - 初始化的提示词集合
    ///          - Initialized prompts collection
    ///
    /// # 注意 (Note)
    /// 如果初始化过程中出现错误，此函数将会panic
    /// This function will panic if there's an error during initialization
    #[deprecated(since = "next_version", note = "请使用返回Result的init函数代替")]
    pub fn init_unchecked() -> Self {
        let (template, info_with_contents) = load().expect("Failed to load prompts");
        let filename_with_prompts = assemble(&template, &info_with_contents);

        Self {
            info_with_contents,
            get_search_keywords: filename_with_prompts["get_search_keywords"].clone(),
            get_paper_scores: filename_with_prompts["get_paper_scores"].clone(),
            get_paper_overview: filename_with_prompts["get_paper_overview"].clone(),
            get_note_with_review: filename_with_prompts["get_note_with_review"].clone(),
            discuss_paper_details: filename_with_prompts["discuss_paper_details"].clone(),
            get_note_with_discussion: filename_with_prompts["get_note_with_discussion"].clone(),
        }
    }
}

/// 单个提示结构体，包含角色提示和阶段提示
/// Single prompt struct containing character prompts and stage prompts
#[derive(Clone, Debug)]
pub struct Prompt {
    /// 角色提示映射，从角色名称到提示内容
    /// Character prompts mapping, from character name to prompt content
    pub character_prompts: HashMap<String, String>,
    
    /// 阶段提示映射，从阶段名称到提示内容
    /// Stage prompts mapping, from stage name to prompt content
    pub stage_prompts: HashMap<String, String>,
}

impl Prompt {
    /// 获取默认角色（assistant）的提示
    /// Get prompt for default character (assistant)
    ///
    /// # 返回 (Returns)
    /// * `Result<String, PromptModelError>` - 成功返回默认角色的提示，失败返回错误
    ///                                      - Returns prompt for default character on success, error on failure
    pub fn default(&self) -> Result<String, PromptModelError> {
        self.character("assistant")
    }
    
    /// 获取默认角色（assistant）的提示（无错误处理版本，保持向后兼容）
    /// Get prompt for default character (assistant) (no error handling version, for backward compatibility)
    ///
    /// # 返回 (Returns)
    /// * `String` - 默认角色的提示
    ///            - Prompt for default character
    ///
    /// # 注意 (Note)
    /// 如果角色提示不存在，此函数将会panic
    /// This function will panic if the character prompt does not exist
    #[deprecated(since = "next_version", note = "请使用返回Result的default函数代替")]
    pub fn default_unchecked(&self) -> String {
        self.character_unchecked("assistant")
    }

    /// 获取指定角色的提示
    /// Get prompt for specified character
    ///
    /// # 参数 (Parameters)
    /// * `character_name` - 角色名称
    ///                    - Character name
    ///
    /// # 返回 (Returns)
    /// * `Result<String, PromptModelError>` - 成功返回指定角色的提示，失败返回错误
    ///                                      - Returns prompt for specified character on success, error on failure
    pub fn character(&self, character_name: &str) -> Result<String, PromptModelError> {
        self.character_prompts
            .get(character_name)
            .cloned()
            .ok_or_else(|| Report::new(PromptModelError::CharacterPromptNotFound(character_name.to_string())))
    }
    
    /// 获取指定角色的提示（无错误处理版本，保持向后兼容）
    /// Get prompt for specified character (no error handling version, for backward compatibility)
    ///
    /// # 参数 (Parameters)
    /// * `character_name` - 角色名称
    ///                    - Character name
    ///
    /// # 返回 (Returns)
    /// * `String` - 指定角色的提示
    ///            - Prompt for specified character
    ///
    /// # 注意 (Note)
    /// 如果角色提示不存在，此函数将会panic
    /// This function will panic if the character prompt does not exist
    #[deprecated(since = "next_version", note = "请使用返回Result的character函数代替")]
    pub fn character_unchecked(&self, character_name: &str) -> String {
        self.character_prompts.get(character_name)
            .expect(&format!("Character prompt not found: {}", character_name))
            .clone()
    }

    /// 获取指定阶段的提示
    /// Get prompt for specified stage
    ///
    /// # 参数 (Parameters)
    /// * `stage_name` - 阶段名称
    ///                - Stage name
    ///
    /// # 返回 (Returns)
    /// * `Result<String, PromptModelError>` - 成功返回指定阶段的提示，失败返回错误
    ///                                      - Returns prompt for specified stage on success, error on failure
    pub fn stage(&self, stage_name: &str) -> Result<String, PromptModelError> {
        self.stage_prompts
            .get(stage_name)
            .cloned()
            .ok_or_else(|| Report::new(PromptModelError::StagePromptNotFound(stage_name.to_string())))
    }
    
    /// 获取指定阶段的提示（无错误处理版本，保持向后兼容）
    /// Get prompt for specified stage (no error handling version, for backward compatibility)
    ///
    /// # 参数 (Parameters)
    /// * `stage_name` - 阶段名称
    ///                - Stage name
    ///
    /// # 返回 (Returns)
    /// * `String` - 指定阶段的提示
    ///            - Prompt for specified stage
    ///
    /// # 注意 (Note)
    /// 如果阶段提示不存在，此函数将会panic
    /// This function will panic if the stage prompt does not exist
    #[deprecated(since = "next_version", note = "请使用返回Result的stage函数代替")]
    pub fn stage_unchecked(&self, stage_name: &str) -> String {
        self.stage_prompts.get(stage_name)
            .expect(&format!("Stage prompt not found: {}", stage_name))
            .clone()
    }
}