// 标准库
use std::collections::HashMap;

// 错误处理
use error_stack::{Report, ResultExt};
use thiserror::Error;

// 辅助工具
use indoc::indoc;

// 项目内部模块
use crate::prompt::model::{Content, Info, Prompt, Template};
use crate::schema::tool_schema::ChatToolSchemaError;

/// 输出描述错误枚举
/// Output description error enum
#[derive(Debug, Error)]
pub enum OutputDescriptionError {
    /// 缺少'json_schema'字段
    /// Missing 'json_schema' field
    #[error("Missing 'json_schema' field")]
    MissingJsonSchemaField,
    
    /// 缺少或无效的'name'字段
    /// Missing or invalid 'name' field
    #[error("Missing or invalid 'name' field")]
    MissingNameField,
    
    /// 缺少或无效的'description'字段
    /// Missing or invalid 'description' field
    #[error("Missing or invalid 'description' field")]
    MissingDescriptionField,
    
    /// 缺少'schema'字段
    /// Missing 'schema' field
    #[error("Missing 'schema' field")]
    MissingSchemaField,
    
    /// 缺少'properties'字段
    /// Missing 'properties' field
    #[error("Missing 'properties' field")]
    MissingPropertiesField,
}

/// 组装模板和内容信息到提示映射中
/// Assemble templates and content information into prompts
///
/// # 参数 (Parameters)
/// * `template` - 模板对象
///               - Template object
/// * `info_with_contents` - 信息与内容的映射
///                        - Mapping between information and content
///
/// # 返回 (Returns)
/// * `HashMap<String, Prompt>` - 名称到提示的映射
///                              - Mapping from names to prompts
pub fn assemble(template: &Template, info_with_contents: &HashMap<Info, Content>) -> HashMap<String, Prompt> {
    let mut result = HashMap::with_capacity(info_with_contents.len());
    
    for (info, content) in info_with_contents {
        let character_prompts = assemble_character_prompt(template, content);
        let stage_prompts = assemble_stage_prompt(content);

        result.insert(info.name.clone(), Prompt {
            character_prompts,
            stage_prompts,
        });
    }
    
    result
}

/// 组装角色提示
/// Assemble character prompts
///
/// # 参数 (Parameters)
/// * `template` - 模板对象
///               - Template object
/// * `content` - 内容对象
///             - Content object
///
/// # 返回 (Returns)
/// * `HashMap<String, String>` - 角色名称到提示文本的映射
///                              - Mapping from character names to prompt texts
fn assemble_character_prompt(template: &Template, content: &Content) -> HashMap<String, String> {
    let tcp = &template.character_prompts;  // 模板角色提示 (template character prompts)
    let ccp = &content.character_prompts;   // 内容角色提示 (content character prompts)
    let num_chars = content.character_prompts.character_names.len();
    let mut result = HashMap::with_capacity(num_chars);

    for character_name in &content.character_prompts.character_names {
        let mut character_prompt_parts = Vec::with_capacity(7); // 预分配空间为可能的元素数量
                                                              // Pre-allocate space for possible elements

        // 处理各个字段
        // Process each field
        let field_pairs = [
            (&tcp.task_description,     &ccp.task_description),
            // (&tcp.input_description,    &ccp.input_description),
            // (&tcp.output_description,   &ccp.output_description),
            (&tcp.principle,            &ccp.principle),
            (&tcp.how_to_think,         &ccp.how_to_think),
            (&tcp.examples,             &ccp.examples),
        ];
        
        for (template_field, content_field) in field_pairs.iter() {
            if let Some(value) = content_field.get(character_name)
                .filter(|value| !value.is_empty())
                .or_else(|| content_field.get("assistant"))
                .filter(|value| !value.is_empty())
            {
                character_prompt_parts.push(build_element(
                    &template_field.element_name,
                    &template_field.description,
                    value,
                ));
            }
        }
        
        // 处理阶段描述
        // Process stage description
        let mut stage_content = String::with_capacity(content.stage_prompt.len() * 50);
        for stage_prompt in &content.stage_prompt {
            stage_content.push_str(&format!("{}: {}\n", stage_prompt.name, stage_prompt.description));
        }
        
        character_prompt_parts.push(build_element(
            &template.character_prompts.stage_description.element_name,
            &template.character_prompts.stage_description.description,
            &stage_content,
        ));
        
        // 合并所有部分
        // Combine all parts
        result.insert(character_name.clone(), character_prompt_parts.join(""));
    }
    
    result
}

/// 构建XML元素
/// Build XML element
///
/// # 参数 (Parameters)
/// * `element_name` - 元素名称
///                   - Element name
/// * `element_description` - 元素描述
///                         - Element description
/// * `content` - 元素内容
///             - Element content
///
/// # 返回 (Returns)
/// * `String` - 格式化的XML元素字符串
///            - Formatted XML element string
#[inline]
fn build_element(element_name: &str, element_description: &str, content: &str) -> String {
    if content.is_empty() {
        String::new()
    } else {
        // 预分配适当的容量
        // Pre-allocate appropriate capacity
        let capacity = element_name.len() * 2 + element_description.len() + content.len() + 20;
        let mut result = String::with_capacity(capacity);
        
        result.push_str("<");
        result.push_str(element_name);
        result.push_str(">\n    <!-- ");
        result.push_str(element_description);
        result.push_str(" -->\n");
        result.push_str(content);
        result.push_str("</");
        result.push_str(element_name);
        result.push_str(">\n");
        
        result
    }
}

/// 组装阶段提示
/// Assemble stage prompts
///
/// # 参数 (Parameters)
/// * `content` - 内容对象
///             - Content object
///
/// # 返回 (Returns)
/// * `HashMap<String, String>` - 阶段名称到提示内容的映射
///                              - Mapping from stage names to prompt contents
#[inline]
fn assemble_stage_prompt(content: &Content) -> HashMap<String, String>{
    let mut result = HashMap::with_capacity(content.stage_prompt.len());
    
    for stage_prompt in &content.stage_prompt {
        result.insert(stage_prompt.name.clone(), stage_prompt.content.clone());
    }
    
    result
}

/// 组装输出描述
/// Assemble output description
///
/// # 参数 (Parameters)
/// * `json_schema` - JSON模式对象
///                 - JSON schema object
///
/// # 返回 (Returns)
/// * `error_stack::Result<String, OutputDescriptionError>` - 成功返回组装后的描述，失败返回错误
///                                                         - Returns assembled description on success, error on failure
pub fn assemble_output_description(
    json_schema: serde_json::Value,
) -> error_stack::Result<String, OutputDescriptionError> {
    // 获取json_schema字段
    // Get json_schema field
    let json_schema = json_schema
        .get("json_schema")
        .ok_or(Report::new(OutputDescriptionError::MissingJsonSchemaField))?;

    // 获取名称
    // Get name
    let name = json_schema
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(OutputDescriptionError::MissingNameField))?;

    // 获取描述
    // Get description
    let description = json_schema
        .get("description")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(OutputDescriptionError::MissingDescriptionField))?;

    // 获取模式和属性
    // Get schema and properties
    let schema = json_schema
        .get("schema")
        .ok_or(Report::new(OutputDescriptionError::MissingSchemaField))?;
    let properties = schema
        .get("properties")
        .ok_or(Report::new(OutputDescriptionError::MissingPropertiesField))?;

    // 构造结果字符串，预先分配容量
    // Construct result string with pre-allocated capacity
    let mut result = String::with_capacity(1024);
    result.push_str("你的回答需要包含以下内容。\n");
    result.push_str(name);
    result.push_str(": ");
    result.push_str(description);
    result.push_str("\n");
    result.push_str(&extract_properties(properties, 1));

    Ok(result)
}

/// 组装工具提示
/// Assemble tools prompt
///
/// # 参数 (Parameters)
/// * `json_schema_vec` - JSON模式对象数组
///                     - Array of JSON schema objects
///
/// # 返回 (Returns)
/// * `error_stack::Result<String, ChatToolSchemaError>` - 成功返回组装后的工具提示，失败返回错误
///                                                      - Returns assembled tools prompt on success, error on failure
pub fn assemble_tools_prompt(json_schema_vec: Vec<serde_json::Value>) -> error_stack::Result<String, ChatToolSchemaError> {
    // 预估工具提示的总大小并预分配容量
    // Estimate total size of tool prompts and pre-allocate capacity
    let mut tools = String::with_capacity(json_schema_vec.len() * 256);

    for json_schema in json_schema_vec {
        tools.push_str(
            &assemble_tool_prompt(json_schema)
                .change_context(ChatToolSchemaError::AssembleToolPrompt)?
        );
    }

    // 为每行添加缩进，优化拼接
    // Add indentation to each line, optimize concatenation
    let mut indented_tools = String::with_capacity(tools.len() + tools.lines().count() * 8);
    for line in tools.lines() {
        indented_tools.push_str("        ");
        indented_tools.push_str(line);
        indented_tools.push('\n');
    }

    // 使用indoc!宏格式化最终结果
    // Format final result using indoc! macro
    let result = format!(
        indoc! {"
            <ToolUse>
                当你需要调用某个工具时，请在回答中使用 <ToolUse></ToolUse> 标签，遵循以下要求：
                1. 每个标签仅包含一个工具调用，且工具的调用必须按照参数要求提供完整信息。
                2. 每个标签内的内容应包含：
                  - 工具名称：如 send_email。
                  - 工具描述：简要描述该工具的功能。
                  - 参数：提供工具所需的所有参数，并确保格式正确（如类型、命名等）。
                3. 你可以在同一回答中使用多个<ToolUse></ToolUse>标签，每个标签对应任意你想要的工具调用。
                4. 我会根据你提供的调用信息执行相应的操作，并将结果返回给你。
                5. 不要在回答中仅包含<ToolUse></ToolUse>标签, 带有一些其他的文字, 可以是你的想法或是其他想表述的内容。\n
                你可以使用以下工具：\n\n{}\n
            </ToolUse>
        "},
        indented_tools // 统一缩进后的工具描述
                      // Tool descriptions with unified indentation
    );

    Ok(result)
}

/// 组装单个工具提示
/// Assemble single tool prompt
///
/// # 参数 (Parameters)
/// * `json_schema` - 工具的JSON模式对象
///                 - JSON schema object for a tool
///
/// # 返回 (Returns)
/// * `error_stack::Result<String, ChatToolSchemaError>` - 成功返回组装后的工具提示，失败返回错误
///                                                      - Returns assembled tool prompt on success, error on failure
fn assemble_tool_prompt(json_schema: serde_json::Value) -> error_stack::Result<String, ChatToolSchemaError> {
    // 提取function对象
    // Extract function object
    let function = json_schema.get("function")
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionField))?;
    
    // 提取函数名和描述
    // Extract function name and description
    let function_name = function.get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionName))?;
    let function_desc = function.get("description")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionDescription))?;

    // 提取parameters对象
    // Extract parameters object
    let parameters = function.get("parameters")
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionParameters))?;

    // 提取properties字段
    // Extract properties field
    let properties = parameters.get("properties")
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionProperties))?;

    // 构造结果字符串，预先分配容量
    // Construct result string with pre-allocated capacity
    let mut result = String::with_capacity(512);
    result.push_str("函数名: ");
    result.push_str(function_name);
    result.push_str("\n函数描述: ");
    result.push_str(function_desc);
    result.push_str("\n");

    // 提取和格式化属性信息
    // Extract and format property information
    result.push_str(&extract_properties(properties, 1));

    Ok(result)
}

/// 提取属性信息
/// Extract property information
///
/// # 参数 (Parameters)
/// * `properties` - 属性对象
///                - Properties object
/// * `indent` - 缩进级别
///            - Indentation level
///
/// # 返回 (Returns)
/// * `String` - 格式化的属性信息字符串
///            - Formatted property information string
pub fn extract_properties(properties: &serde_json::Value, indent: usize) -> String {
    // 预估属性数量，为结果字符串分配合理容量
    // Estimate number of properties and allocate reasonable capacity
    let props_len = properties.as_object().map_or(0, |obj| obj.len());
    let mut result = String::with_capacity(props_len * 128);
    let indent_str = "  ".repeat(indent);

    if let Some(props) = properties.as_object() {
        for (prop_name, prop_value) in props {
            // 跳过"cot"属性
            // Skip "cot" property
            if prop_name == "cot" {
                continue;
            }
            
            // 创建基本属性行，预先分配容量
            // Create basic property line with pre-allocated capacity
            let mut line = String::with_capacity(prop_name.len() + 100);
            line.push_str(&indent_str);
            line.push_str(prop_name);

            // 提取常用字段为局部变量
            // Extract commonly used fields as local variables
            let prop_type = prop_value.get("type");
            let prop_desc = prop_value.get("description").and_then(|d| d.as_str());
            let prop_enum = prop_value.get("enum");

            // 添加类型信息
            // Add type information
            if let Some(type_val) = prop_type {
                match type_val {
                    serde_json::Value::String(type_str) => {
                        line.push_str(" (");
                        line.push_str(type_str);
                        line.push_str(")");
                    }
                    serde_json::Value::Array(type_array) => {
                        let mut types = Vec::with_capacity(type_array.len());
                        for v in type_array {
                            if let Some(s) = v.as_str() {
                                types.push(s.to_string());
                            }
                        }
                        if !types.is_empty() {
                            line.push_str(" ([");
                            line.push_str(&types.join(", "));
                            line.push_str("])");
                        }
                    }
                    _ => {}
                }
            }

            // 添加描述信息
            // Add description information
            if let Some(desc) = prop_desc {
                line.push_str(": ");
                line.push_str(desc);
            }

            // 添加枚举信息
            // Add enum information
            if let Some(enum_val) = prop_enum {
                if let Some(enum_values) = enum_val.as_array() {
                    let mut enum_strings = Vec::with_capacity(enum_values.len());
                    for v in enum_values {
                        if let Some(s) = v.as_str() {
                            enum_strings.push(s.to_string());
                        }
                    }
                    if !enum_strings.is_empty() {
                        line.push_str(" (Enum: [");
                        line.push_str(&enum_strings.join(", "));
                        line.push_str("])");
                    }
                }
            }

            // 添加属性行到结果
            // Add property line to result
            line.push('\n');
            result.push_str(&line);

            // 递归处理嵌套对象
            // Recursively process nested objects
            if prop_type == Some(&serde_json::Value::String("object".to_string())) {
                if let Some(sub_properties) = prop_value.get("properties") {
                    result.push_str(&extract_properties(sub_properties, indent + 1));
                }
            }
        }
    }

    result
}