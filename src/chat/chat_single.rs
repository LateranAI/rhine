// 外部库引用 / External library imports (按泛用程度从高到低排序 / ordered by generality from high to low)
// 基础数据类型和序列化 / Basic data types and serialization
use serde::de::DeserializeOwned;
use serde_json::json;

// 错误处理 / Error handling
use thiserror::Error;
use error_stack::{Report, Result, ResultExt};

// 异步运行时和流处理 / Async runtime and stream processing
use tokio::task;

// 日志记录 / Logging
use tracing::log::{info};

// 本地库引用 / Local library imports
use crate::chat::chat_base::{BaseChat, ChatError};
use crate::chat::chat_tool::{ChatTool};
use crate::chat::message::Role;
use crate::config::ModelCapability;
use crate::prompt::assembler::{assemble_output_description, assemble_tools_prompt};
use crate::schema::json_schema::JsonSchema;
use crate::schema::tool_schema::extract_tool_uses;


/// 工具调用错误枚举
/// Tool call error enumeration
#[derive(Debug, Error)]
pub enum ToolCallError {
    /// 解析函数调用失败
    /// Failed to parse function call
    #[error("Failed to parse function call")]
    ParseFunctionCall,
    
    /// 函数未找到
    /// Function not found
    #[error("Function '{0}' not found")]
    FunctionNotFound(String),
    
    /// 函数执行失败
    /// Failed to execute function
    #[error("Failed to execute function '{0}'")]
    FunctionExecution(String),
    
    /// 序列化结果失败
    /// Failed to serialize result
    #[error("Failed to serialize function result")]
    SerializeResult,
    
    /// 反序列化参数失败
    /// Failed to deserialize arguments
    #[error("Failed to deserialize arguments: {0}")]
    DeserializeArguments(String),
    
    /// 获取 JSON 失败
    /// Failed to get JSON
    #[error("Failed to get json: {0}")]
    GetJson(String),
    
    /// 从响应中提取函数调用失败
    /// Failed to extract function call from response
    #[error("Failed to extract function call from: {0}")]
    ExtractFunctionCall(String),
    
    /// 缺少字段
    /// Missing field
    #[error("Missing field: {0}")]
    MissingField(String),
}


/// 单聊天会话结构体，扩展基础聊天功能，支持工具调用
/// Single chat session structure, extends basic chat functionality with tool calling support
#[derive(Debug, Clone)]
pub struct SingleChat {
    /// 基础聊天实例
    /// Base chat instance
    pub base: BaseChat,
    /// 是否需要流式响应
    /// Whether streaming response is needed
    need_stream: bool,
    /// 工具模式配置
    /// Tool schema configuration
    tools_schema: Vec<serde_json::Value>,
}

impl SingleChat {
    /// 使用 API 名称创建新的单聊天会话
    /// 
    /// Create a new single chat session with API name
    ///
    /// # 参数 / Parameters
    /// * `api_name` - API 名称 / API name
    /// * `character_prompt` - 角色提示词 / Character prompt
    /// * `need_stream` - 是否需要流式响应 / Whether streaming response is needed
    ///
    /// # 返回 / Returns
    /// * `Self` - 新创建的 SingleChat 实例 / Newly created SingleChat instance
    pub fn new_with_api_name(api_name: &str, character_prompt: &str, need_stream: bool) -> Self {
        let base = BaseChat::new_with_api_name(api_name, character_prompt, need_stream);
        Self {
            base,
            need_stream,
            tools_schema: Vec::new(),
        }
    }

    /// 使用模型能力创建新的单聊天会话
    /// 
    /// Create a new single chat session with model capability
    ///
    /// # 参数 / Parameters
    /// * `model_capability` - 模型能力枚举 / Model capability enum
    /// * `character_prompt` - 角色提示词 / Character prompt
    /// * `need_stream` - 是否需要流式响应 / Whether streaming response is needed
    ///
    /// # 返回 / Returns
    /// * `Self` - 新创建的 SingleChat 实例 / Newly created SingleChat instance
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

    /// 获取用户输入的回答
    /// 
    /// Get answer for user input
    ///
    /// # 参数 / Parameters
    /// * `user_input` - 用户输入 / User input
    ///
    /// # 返回 / Returns
    /// * `Result<String, ChatError>` - 回答结果 / Answer result
    pub async fn get_answer_with_end_path(&mut self, end_path: &[usize], user_input: &str) -> Result<String, ChatError> {
        // 添加用户消息
        // Add user message
        self.base.add_message(Role::User, user_input);
        let request_body = self.base.build_request_body(end_path, &Role::User);

        let content = if self.need_stream {
            // 使用流式响应
            // Use streaming response
            let (stream, semaphore_permit) = self
                .base
                .get_stream_response(request_body)
                .await?;
            BaseChat::get_content_from_stream_resp(stream, semaphore_permit).await?
        } else {
            // 使用普通响应
            // Use normal response
            let response = self
                .base
                .get_response(request_body)
                .await?;
            BaseChat::get_content_from_resp(&response)?
        };

        info!("GetLLMAPIAnswer: {}", content);
        // 添加助手消息
        // Add assistant message
        self.base.add_message(Role::Assistant, &content);
        Ok(content)
    }

    pub async fn get_answer(&mut self, user_input: &str) -> Result<String, ChatError> {
        let end_path = self.base.message_path.clone();
        // 获取回答
        // Get answer
        self.get_answer_with_end_path(end_path.as_ref(), user_input).await
    }

    /// 获取结构化 JSON 格式的回答
    /// 
    /// Get structured JSON answer
    ///
    /// # 参数 / Parameters
    /// * `user_input` - 用户输入 / User input
    ///
    /// # 返回 / Returns
    /// * `Result<T, ChatError>` - 结构化回答结果 / Structured answer result
    pub async fn get_json_answer<T: DeserializeOwned + 'static + JsonSchema>(
        &mut self,
        user_input: &str,
    ) -> Result<T, ChatError> {
        // 获取 JSON 模式
        // Get JSON schema
        let schema = T::json_schema();
        
        // 添加输出描述系统消息
        // Add output description system message
        self.base.add_message(
            Role::System,
            assemble_output_description(schema.clone())
                .change_context(ChatError::AssembleOutputDescriptionError)?
                .as_str(),
        );
        
        // 获取回答
        // Get answer
        let answer = self.get_answer(user_input).await?;
        info!("GetLLMAPIAnswer: {}", answer);

        // 解析 JSON 回答
        // Parse JSON answer
        ChatTool::get_json::<T>(&answer, schema).await
    }

    /// 设置工具模式
    /// 
    /// Set tool schema
    ///
    /// # 参数 / Parameters
    /// * `tools_schema` - 工具模式配置 / Tool schema configuration
    pub fn set_tools(&mut self, tools_schema: Vec<serde_json::Value>) {
        self.tools_schema = tools_schema.clone();
        
        // 组装工具提示
        // Assemble tools prompt
        let tools_prompt = assemble_tools_prompt(tools_schema).unwrap(); // assemble_tools_prompt 目前没有错误，所以暂时保留 / Currently there's no error in assemble_tools_prompt, so keep it for now
        
        // 添加工具提示系统消息
        // Add tools prompt system message
        self.base.add_message(Role::System, &tools_prompt);
    }

    /// 处理单个工具调用
    /// 
    /// Process a single tool call
    ///
    /// # 参数 / Parameters
    /// * `text_call` - 函数调用文本 / Function call text
    /// * `tools_schema` - 工具模式配置 / Tool schema configuration
    ///
    /// # 返回 / Returns
    /// * `Result<String, Report<ToolCallError>>` - 处理结果 / Processing result
    async fn process_tool_call(
        text_call: String, 
        tools_schema: Vec<serde_json::Value>
    ) -> error_stack::Result<String, ToolCallError> {
        // 解析函数调用
        // Parse function call
        let function_call: serde_json::Value = ChatTool::get_function(&text_call, json!({"tools": tools_schema}))
            .await
            .change_context(ToolCallError::ParseFunctionCall)?;
        info!("function_call: {}", serde_json::to_string_pretty(&function_call).unwrap());


        // 提取调用参数
        // Extract call parameters
        let function_name = function_call["name"].as_str()
            .ok_or_else(|| Report::new(ToolCallError::MissingField("name".to_string())))?;
            
        let arg_str = function_call["arguments"].as_str()
            .ok_or_else(|| Report::new(ToolCallError::MissingField("arguments".to_string())))?;
            
        let arg_json: serde_json::Value = serde_json::from_str(arg_str)
            .map_err(|e| Report::new(ToolCallError::DeserializeArguments(e.to_string()))
            .attach_printable(format!("Failed to deserialize arguments for function: {}", function_name)))?;

        // 调用函数
        // Call function
        use crate::schema::tool_schema::get_tool_registry;
        let registry = get_tool_registry();

        match registry.get(function_name) {
            Some(tool_fn) => {
                info!("Calling function named: {}", function_name);
                match tool_fn(arg_json.clone()) {
                    Ok(result) => {
                        let serialized = serde_json::to_string_pretty(&result)
                            .map_err(|_| Report::new(ToolCallError::SerializeResult))?;
                        info!("Calling function succeeded: {}", serialized);
                        Ok(serialized)
                    }
                    Err(e) => {
                        let err_msg = format!("Calling function '{}' failed: {}", function_name, e);
                        info!("{}", err_msg);
                        Ok(err_msg)
                    }
                }
            }
            None => {
                let err_msg = format!("Cannot find function named '{}'", function_name);
                info!("{}", err_msg);
                Ok(err_msg)
            }
        }
    }

    /// 获取工具调用回答
    /// 
    /// Get tool call answer
    ///
    /// # 参数 / Parameters
    /// * `user_input` - 用户输入 / User input
    ///
    /// # 返回 / Returns
    /// * `Result<(String, Vec<String>), ToolCallError>` - 清理后的回答和工具调用结果 / Cleaned answer and tool call results
    pub async fn get_tool_answer(
        &mut self,
        user_input: &str,
    ) -> Result<(String, Vec<String>), ToolCallError> {
        // 获取包含函数调用的回答
        // Get answer with function calls
        let answer_with_text_calls = self.get_answer(
            user_input,
        ).await.map_err(|e| {
            Report::new(ToolCallError::ExtractFunctionCall(format!(
                "Failed to get answer: {:?}",
                e
            )))
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
            .fold(answer_with_text_calls, |acc, call| {
                acc.replace(&format!("<ToolUse>{}</ToolUse>", call), "")
            });
        info!("clean_answer: {}", clean_answer);

        // 创建工具模式的副本用于任务间共享
        // Create a copy of the tool schema for sharing between tasks
        let tools_schema = self.tools_schema.clone();

        // 创建任务
        // Create tasks
        let tasks = text_calls.into_iter().map(|text_call| {
            let tools_schema_clone = tools_schema.clone();
            task::spawn(async move {
                Self::process_tool_call(text_call, tools_schema_clone).await
            })
        }).collect::<Vec<_>>();

        // 并行等待所有任务完成
        // Wait for all tasks to complete in parallel
        for task in tasks {
            match task.await {
                Ok(result) => {
                    match result {
                        Ok(success_result) => results.push(success_result),
                        Err(err) => return Err(err) // 正确地传播错误
                    }
                },
                Err(e) => {
                    return Err(Report::new(ToolCallError::FunctionExecution(format!(
                        "Task join error: {:?}", e
                    ))));
                }
            }
        }

        Ok((clean_answer, results))
    }
}