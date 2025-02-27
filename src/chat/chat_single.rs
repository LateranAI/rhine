use bytes::Bytes;
use crate::chat::chat_base::{BaseChat, ChatError, Role};
use crate::chat::chat_tool::{ChatTool};
use crate::config::ModelCapability;
use crate::prompt::assembler::{assemble_output_description, assemble_tools_prompt};
use crate::schema::json_schema::JsonSchema;
use crate::schema::tool_schema::extract_tool_uses;
use error_stack::{Report, Result, ResultExt};
use futures::executor::block_on;
use futures::{Stream, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use serde_json::json;
use thiserror::Error;
use tokio::sync::OwnedSemaphorePermit;
use tracing::log::{debug, info};


#[derive(Debug, Error)]
pub enum ToolCallError {
    #[error("Failed to parse function call")]
    ParseFunctionCall,
    #[error("Function '{0}' not found")]
    FunctionNotFound(String),
    #[error("Failed to execute function '{0}'")]
    FunctionExecution(String),
    #[error("Failed to serialize function result")]
    SerializeResult,
    #[error("Failed to deserialize arguments: {0}")]
    DeserializeArguments(String),
    #[error("Failed to get json: {0}")]
    GetJson(String),
    #[error("Failed to extract function call from: {0}")]
    ExtractFunctionCall(String),
}



#[derive(Debug, Clone)]
pub struct SingleChat {
    pub base: BaseChat,
    need_stream: bool,
    tools_schema: Vec<serde_json::Value>,
}

impl SingleChat {
    pub fn new_with_api_name(api_name: &str, character_prompt: &str, need_stream: bool) -> Self {
        let base = BaseChat::new_with_api_name(api_name, character_prompt, need_stream);
        Self {
            base,
            need_stream,
            tools_schema: Vec::new(),
        }
    }

    pub fn new_with_model_capability(
        model_capability: ModelCapability,
        character_prompt: &str,
        need_stream: bool,
    ) -> Self {
        let base =
            BaseChat::new_with_model_capability(model_capability, character_prompt, need_stream);
        Self {
            base,
            need_stream,
            tools_schema: Vec::new(),
        }
    }

    pub async fn get_answer(&mut self, user_input: &str) -> Result<String, ChatError> {
        self.base.add_message(Role::User, user_input);
        let request_body = self.base.build_request_body();

        let mut content= String::new();

        if self.need_stream {
            let (stream, semaphore_permit) = self
                .base
                .get_stream_response(request_body)
                .await?;
            content = BaseChat::get_content_from_stream_resp(stream, semaphore_permit).await?;

        } else {
            let response = self
            .base
            .get_response(request_body)
            .await?;
            content = BaseChat::get_content_from_resp(&response)?;
        }

        info!("GetLLMAPIAnswer: {}", content);
        self.base.add_message(Role::Assistant, content.as_str());
        Ok(content)
    }

    pub async fn get_json_answer<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
        user_input: &str,
    ) -> Result<T, ChatError> {
        let schema = T::json_schema();
        self.base.add_message(
            Role::System,
            assemble_output_description(schema.clone())
                .change_context(ChatError::AssembleOutputDescriptionError)?
                .as_str(),
        );
        let answer = self.get_answer(user_input).await?;
        info!("GetLLMAPIAnswer: {}", answer);

        ChatTool::get_json::<T>(answer.as_str(), schema).await
    }

    pub fn set_tools(&mut self, tools_schema: Vec<serde_json::Value>) {
        self.tools_schema = tools_schema.clone();
        let tools_prompt = assemble_tools_prompt(tools_schema).unwrap(); //assemble_tools_prompt目前没有错误，所以暂时保留
        self.base.add_message(Role::System, tools_prompt.as_str());
    }

    pub async fn get_tool_answer(
        &mut self,
        user_input: &str,
    ) -> Result<(String, Vec<String>), ToolCallError> {
        let answer_with_text_calls = self.get_answer(user_input).await.map_err(|e| {
            Report::new(ToolCallError::ExtractFunctionCall(format!(
                "Failed to get answer: {:?}",
                e
            )))
        })?;

        // 提取原始函数调用文本
        let text_calls = extract_tool_uses(&answer_with_text_calls);

        // 过滤掉函数调用标签后的纯文本回答
        let clean_answer = text_calls
            .iter()
            .fold(answer_with_text_calls.clone(), |acc, call| {
                acc.replace(&format!("<FunctionCalling>{}</FunctionCalling>", call), "")
            });

        let mut results = Vec::new();

        for text_call in text_calls {
            // 解析函数调用
            let function_call: serde_json::Value = match block_on(async {
                ChatTool::get_function(text_call.as_str(), json!({"tools": self.tools_schema}))
                    .await
                    .change_context(ToolCallError::ParseFunctionCall)
            }) {
                Ok(v) => {
                    println!("v: {}", v);
                    v
                }
                Err(report) => {
                    //直接把error_stack的report转换成string
                    results.push(format!("Function Calling parsing error: {:?}", report));
                    continue;
                }
            };

            // 提取调用参数
            let function_name = function_call["name"].as_str().unwrap_or("unknown_function");
            let arg_str = function_call["arguments"].as_str().unwrap();
            let arg_json: serde_json::Value = serde_json::from_str(arg_str).map_err(|e| {
                Report::new(ToolCallError::DeserializeArguments(e.to_string())).attach_printable(
                    format!(
                        "Failed to deserialize arguments for function: {}",
                        function_name
                    ),
                )
            })?;

            // 调用函数
            use crate::schema::tool_schema::get_tool_registry;
            let registry = get_tool_registry();

            match registry.get(function_name) {
                Some(tool_fn) => {
                    println!("Calling function named: {}", function_name);
                    match tool_fn(arg_json.clone()) {
                        Ok(result) => {
                            let serialized = serde_json::to_string_pretty(&result)
                                .map_err(|_| Report::new(ToolCallError::SerializeResult))?;
                            println!("Calling function succeeded: {}", serialized);
                            results.push(serialized);
                        }
                        Err(e) => {
                            let err_msg =
                                format!("Calling function'{}'failed: {}", function_name, e);
                            println!("{}", err_msg);
                            results.push(err_msg); // 这里可以根据实际情况决定是否需要将错误信息转换为 Report
                        }
                    }
                }
                None => {
                    let err_msg = format!("Cannot find function named '{}'", function_name);
                    println!("{}", err_msg);
                    results.push(err_msg);
                }
            }
        }
        Ok((clean_answer, results))
    }
}
