// 外部库引用 / External library imports (按泛用程度从高到低排序 / ordered by generality from high to low)
// 基础数据类型和序列化 / Basic data types and serialization
use serde::de::DeserializeOwned;
use serde_json::json;

// 错误处理 / Error handling
use error_stack::{Report, Result, ResultExt};
use thiserror::Error;

// 异步运行时和流处理 / Async runtime and stream processing
use tokio::task;
// 日志记录 / Logging
use tracing::log::info;

// 本地库引用 / Local library imports
use crate::chat::chat_base::{BaseChat, ChatError};
use crate::chat::chat_tool::ChatTool;
use crate::chat::message::Role;
use crate::config::ModelCapability;
use crate::prompt::assembler::{assemble_output_description, assemble_tools_prompt};
use crate::schema::json_schema::JsonSchema;
use crate::schema::tool_schema::extract_tool_uses;

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

    #[error("Missing field: {0}")]
    MissingField(String),
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

    pub async fn get_resp_with_new_question(
        &mut self,
        parent_path: &[usize],
        user_input: &str,
    ) -> Result<serde_json::Value, ChatError> {
        self.base
            .add_message_with_parent_path(parent_path, Role::User, user_input)?;
        Ok(self
            .base
            .build_request_body(&self.base.session.default_path.clone(), &Role::User)?)
    }

    pub async fn get_resp_again(
        &mut self,
        end_path: &[usize],
    ) -> Result<serde_json::Value, ChatError> {
        Ok(self.base.build_request_body(end_path, &Role::User)?)
    }

    pub async fn get_resp(&mut self, user_input: &str) -> Result<serde_json::Value, ChatError> {
        info!("path: {:?}", self.base.session.default_path.clone());
        self.get_resp_with_new_question(&self.base.session.default_path.clone(), user_input)
            .await
    }

    async fn get_content_from_resp(
        &mut self,
        request_body: serde_json::Value,
    ) -> Result<String, ChatError> {
        let content = if self.need_stream {
            // 使用流式响应
            // Use streaming response
            let (stream, semaphore_permit) = self
                .base
                .get_stream_response(request_body.clone())
                .await
                .attach_printable("Failed to get stream response")?;

            BaseChat::get_content_from_stream_resp(stream, semaphore_permit)
                .await
                .attach_printable("Failed to extract content from stream response")?
        } else {
            // 使用普通响应
            // Use normal response
            let response = self
                .base
                .get_response(request_body.clone())
                .await
                .attach_printable("Failed to get response")?;

            BaseChat::get_content_from_resp(&response)
                .attach_printable("Failed to extract content from response")?
        };

        info!("GetLLMAPIAnswer: {}", content);
        // 添加助手消息
        // Add assistant message
        self.base.add_message(Role::Assistant, &content)?;
        Ok(content)
    }

    pub async fn get_json_answer<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
        user_input: &str,
    ) -> Result<T, ChatError> {
        // 获取 JSON 模式
        // Get JSON schema
        let schema = T::json_schema();

        // 添加输出描述系统消息
        // Add output description system message
        let output_description = assemble_output_description(schema.clone())
            .change_context(ChatError::AssembleOutputDescriptionError)
            .attach_printable(format!(
                "Failed to assemble output description for schema: {:?}",
                serde_json::to_string(&schema)
                    .unwrap_or_else(|_| "Schema serialization failed".to_string())
            ))?;

        self.base
            .add_message(Role::System, output_description.as_str())?;

        // 获取回答
        // Get answer
        let resp = self
            .get_resp(user_input)
            .await
            .attach_printable("Failed to get answer for JSON request")?;

        let answer = self.get_content_from_resp(resp).await?;

        // 解析 JSON 回答
        // Parse JSON answer
        ChatTool::get_json::<T>(&answer, schema)
            .await
            .attach_printable(format!("Failed to parse answer as JSON: {}", answer))
    }

    pub fn set_tools(&mut self, tools_schema: Vec<serde_json::Value>) -> Result<(), ChatError> {
        self.tools_schema = tools_schema.clone();

        // 组装工具提示
        // Assemble tools prompt
        let tools_prompt = assemble_tools_prompt(tools_schema).unwrap(); // assemble_tools_prompt 目前没有错误，所以暂时保留 / Currently there's no error in assemble_tools_prompt, so keep it for now

        // 添加工具提示系统消息
        // Add tools prompt system message
        self.base.add_message(Role::System, &tools_prompt)
    }

    async fn process_tool_call(
        text_call: String,
        tools_schema: Vec<serde_json::Value>,
    ) -> error_stack::Result<String, ToolCallError> {
        // 解析函数调用
        // Parse function call
        let function_call: serde_json::Value =
            ChatTool::get_function(&text_call, json!({"tools": tools_schema}))
                .await
                .change_context(ToolCallError::ParseFunctionCall)
                .attach_printable(format!(
                    "Failed to parse function call from text: {}",
                    text_call
                ))?;

        info!(
            "function_call: {}",
            serde_json::to_string_pretty(&function_call).unwrap_or_default()
        );

        // 提取调用参数
        // Extract call parameters
        let function_name = function_call["name"].as_str().ok_or_else(|| {
            Report::new(ToolCallError::MissingField("name".to_string())).attach_printable(format!(
                "Function call missing 'name' field: {}",
                serde_json::to_string(&function_call).unwrap_or_default()
            ))
        })?;

        let arg_str = function_call["arguments"].as_str().ok_or_else(|| {
            Report::new(ToolCallError::MissingField("arguments".to_string())).attach_printable(
                format!(
                    "Function call missing 'arguments' field for function: {}",
                    function_name
                ),
            )
        })?;

        let arg_json: serde_json::Value = serde_json::from_str(arg_str).map_err(|e| {
            Report::new(ToolCallError::DeserializeArguments(e.to_string())).attach_printable(
                format!(
                    "Failed to deserialize arguments for function '{}': {}",
                    function_name, arg_str
                ),
            )
        })?;

        // 调用函数
        // Call function
        use crate::schema::tool_schema::get_tool_registry;
        let registry = get_tool_registry();

        match registry.get(function_name) {
            Some(tool_fn) => {
                info!("Calling function named: {}", function_name);
                match tool_fn(arg_json.clone()) {
                    Ok(result) => {
                        let serialized = serde_json::to_string_pretty(&result).map_err(|e| {
                            Report::new(ToolCallError::SerializeResult).attach_printable(format!(
                                "Failed to serialize result for function '{}': {:?}",
                                function_name, e
                            ))
                        })?;

                        info!("Calling function succeeded: {}", serialized);
                        Ok(serialized)
                    }
                    Err(e) => {
                        let err_msg = format!("Calling function '{}' failed: {}", function_name, e);
                        info!("{}", err_msg);
                        Ok(err_msg) // 返回错误信息作为可处理的结果而不是抛出异常
                        // Return error message as processable result instead of throwing exception
                    }
                }
            }
            None => {
                let err_msg = format!("Cannot find function named '{}'", function_name);
                info!("{}", err_msg);
                Ok(err_msg) // 同样，返回错误信息而不是抛出异常
                // Similarly, return error message instead of throwing exception
            }
        }
    }

    pub async fn get_tool_answer(
        &mut self,
        user_input: &str,
    ) -> Result<(String, Vec<String>), ToolCallError> {
        // 获取包含函数调用的回答
        // Get answer with function calls
        let resp_with_text_calls = self.get_resp(user_input).await.map_err(|e| {
            Report::new(ToolCallError::ExtractFunctionCall(format!(
                "Failed to get answer for tool call: {:?}",
                e
            )))
            .attach_printable(format!("User input: {}", user_input))
        })?;
        let answer_with_text_calls = self
            .get_content_from_resp(resp_with_text_calls)
            .await
            .map_err(|e| {
                Report::new(ToolCallError::ExtractFunctionCall(format!(
                    "Failed to get answer for tool call: {:?}",
                    e
                )))
                .attach_printable(format!("User input: {}", user_input))
            })?;

        // 提取原始函数调用文本
        // Extract original function call texts
        let text_calls = extract_tool_uses(&answer_with_text_calls);
        info!("text_calls: {:?}", text_calls);

        // 预分配结果向量
        // Pre-allocate result vector
        let mut results = Vec::with_capacity(text_calls.len());

        if text_calls.is_empty() {
            // 如果没有函数调用，直接返回原始回答
            // If there are no function calls, return the original answer
            info!("No function calls found, returning original answer");
            return Ok((answer_with_text_calls, results));
        }

        // 过滤掉函数调用标签后的纯文本回答
        // Filter out pure text answer after removing function call tags
        let clean_answer = text_calls
            .iter()
            .fold(answer_with_text_calls.clone(), |acc, call| {
                acc.replace(&format!("<ToolUse>{}</ToolUse>", call), "")
            });
        info!("clean_answer: {}", clean_answer);

        // 创建工具模式的副本用于任务间共享
        // Create a copy of the tool schema for sharing between tasks
        let tools_schema = self.tools_schema.clone();

        // 创建任务，并行处理所有工具调用
        // Create tasks to process all tool calls in parallel
        let tasks = text_calls
            .into_iter()
            .map(|text_call| {
                let tools_schema_clone = tools_schema.clone();
                task::spawn(
                    async move { Self::process_tool_call(text_call, tools_schema_clone).await },
                )
            })
            .collect::<Vec<_>>();

        // 收集任务执行过程中的错误
        // Collect errors during task execution
        let mut errors = Vec::new();

        // 并行等待所有任务完成
        // Wait for all tasks to complete in parallel
        for (i, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(result) => {
                    match result {
                        Ok(success_result) => results.push(success_result),
                        Err(err) => {
                            // 收集错误但继续处理其他调用
                            // Collect error but continue processing other calls
                            errors.push(format!("Tool call #{} failed: {}", i, err));
                            // 添加错误占位符到结果中
                            // Add error placeholder to results
                            results.push(format!(
                                "{{\"error\": \"Tool call failed with error: {}\"}}",
                                err
                            ));
                        }
                    }
                }
                Err(e) => {
                    // 收集任务执行错误但继续处理其他调用
                    // Collect task execution error but continue processing other calls
                    let error_msg = format!("Task join error for call #{}: {:?}", i, e);
                    errors.push(error_msg.clone());
                    // 添加错误占位符到结果中
                    // Add error placeholder to results
                    results.push(format!(
                        "{{\"error\": \"Task execution failed: {}\"}}",
                        error_msg
                    ));
                }
            }
        }

        // 如果有错误，记录到日志但不中断流程
        // If there are errors, log them but don't interrupt the flow
        if !errors.is_empty() {
            info!("Tool call errors occurred: {:?}", errors);
        }

        Ok((clean_answer, results))
    }
}
