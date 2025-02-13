pub trait JsonSchema {
    fn json_schema() -> serde_json::Value;
}
