use std::collections::HashMap;
use serde::Deserialize;
use crate::prompt::assembler::assemble;
use crate::prompt::loader::load;

// 配置文件结构定义 -----------------------------------------
#[derive(Debug, Deserialize)]
pub struct Config {
    pub template_path: String,
    pub prompt_info: Vec<Info>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
pub struct Info {
    pub name: String,
    description: String,
    pub path: String,
}

// TOML模板结构定义 ----------------------------------------
#[derive(Debug, Deserialize)]
pub struct Template {
    pub character_prompts: CharacterPromptsTemplate,
}

#[derive(Debug, Deserialize)]
pub struct CharacterPromptsTemplate {
    pub task_description: TemplateElement,
    pub stage_description: TemplateElement,
    pub input_description: TemplateElement,
    pub output_description: TemplateElement,
    pub principle: TemplateElement,
    pub how_to_think: TemplateElement,
    pub examples: TemplateElement,
}

#[derive(Debug, Deserialize, Default)]
pub struct TemplateElement {
    pub element_name: String,
    pub description: String,
}

// TOML内容结构定义 ----------------------------------------


#[derive(Clone, Debug, Deserialize, Default)]
pub struct Content {
    pub character_prompts: CharacterPrompts,
    #[serde(default)]
    pub stage_prompt: Vec<StagePrompt>
}
fn default_character_names() -> Vec<String> {
    vec!["assistant".to_string()]
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct CharacterPrompts {
    #[serde(default = "default_character_names")]
    pub character_names: Vec<String>,
    #[serde(default)]
    pub task_description: HashMap<String, String>,
    // #[serde(default)]
    // pub input_description: HashMap<String, String>,
    // #[serde(default)]
    // pub output_description: HashMap<String, String>,
    #[serde(default)]
    pub principle: HashMap<String, String>,
    #[serde(default)]
    pub how_to_think: HashMap<String, String>,
    #[serde(default)]
    pub examples: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct StagePrompt {
    pub name: String,
    pub description: String,
    pub content: String,
}


// 提示词结构体定义 -----------------------------------------

#[derive(Clone, Debug)]
pub struct Prompts {
    pub info_with_contents: HashMap<Info, Content>,
    pub get_search_keywords: Prompt,
    pub get_paper_scores: Prompt,
    pub get_paper_overview: Prompt,
    pub get_note_with_review: Prompt,
    pub discuss_paper_details: Prompt,
    pub get_note_with_discussion: Prompt,
}

impl Prompts {
    pub fn init() -> Self {
        let (template, info_with_contents) = load();
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

#[derive(Clone, Debug)]
pub struct Prompt {
    pub character_prompts: HashMap<String, String>,
    pub stage_prompts: HashMap<String, String>,
}

impl Prompt {
    pub fn default(&self) -> String {
        self.character("assistant")
    }

    pub fn character(&self, character_name: &str) -> String {
        self.character_prompts.get(character_name).unwrap().clone()
    }

    pub fn stage(&self, stage_name: &str) -> String {
        self.stage_prompts.get(stage_name).unwrap().clone()
    }
}

