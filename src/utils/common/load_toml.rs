use std::fs;
use serde::de::DeserializeOwned;
use error_stack::{Result, ResultExt};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadTomlError {
    #[error("Failed to read file")]
    ReadError,

    #[error("Failed to parse TOML content")]
    ParseError,
}

pub fn load_toml<T: DeserializeOwned + 'static>(path: &str) -> Result<T, LoadTomlError> {
    let content = fs::read_to_string(path)
        .change_context(LoadTomlError::ReadError)
        .attach_printable_lazy(|| format!("Failed to read file at path: {path}"))?;

    toml::from_str(&content)
        .change_context(LoadTomlError::ParseError)
        .attach_printable_lazy(|| format!("Invalid TOML format in file: {path}"))
}