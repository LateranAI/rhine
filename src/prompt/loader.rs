use std::collections::HashMap;
use crate::prompt::model::{Config, Content, Info, Template};
use crate::utils::common::load_toml::load_toml;

pub fn load() -> (Template, HashMap<Info, Content>) {
    let config: Config = load_toml("data/prompts/config.toml").unwrap();
    let template: Template = load_toml(&config.template_path).unwrap();

    let info_with_contents: HashMap<Info, Content> = config.prompt_info.iter().map(|info| {
        (info.clone(), load_toml(info.path.as_str()).unwrap())
    }).collect();

    (template, info_with_contents)
}

