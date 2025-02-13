use once_cell::sync::Lazy;
use crate::prompt::model::Prompts;

pub mod model;
pub mod assembler;
pub mod loader;

pub static PROMPTS: Lazy<Prompts> = Lazy::new(Prompts::init);