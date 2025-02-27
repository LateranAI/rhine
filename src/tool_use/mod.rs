use std::collections::HashMap;
use dashmap::DashMap;
use once_cell::sync::Lazy;

pub mod text;
pub mod search;
pub mod browse;
pub mod cmd;
pub mod code;


pub struct Environment {
    text: DashMap<String, String>,
    note: DashMap<String, String>,
}

pub fn add_env(key: &str) {
    ENV_POOL.insert(key.to_string(), Environment {
        text: DashMap::new(),
        note: DashMap::new(),
    });
}


pub fn remove_env(key: &str) {
    let env = ENV_POOL.get(key).unwrap();
    ENV_POOL.remove(key);
}

pub static ENV_POOL: Lazy<DashMap<String, Environment>> = Lazy::new(|| DashMap::new());
