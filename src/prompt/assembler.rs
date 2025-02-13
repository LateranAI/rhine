use std::collections::HashMap;
use error_stack::{Report, ResultExt};
use indoc::indoc;
use thiserror::Error;
use crate::prompt::model::{Content, Info, Prompt, Template};
use crate::schema::tool_schema::ChatToolSchemaError;


#[derive(Debug, Error)]
pub enum OutputDescriptionError {
    #[error("Missing 'json_schema' field")]
    MissingJsonSchemaField,
    #[error("Missing or invalid 'name' field")]
    MissingNameField,
    #[error("Missing or invalid 'description' field")]
    MissingDescriptionField,
    #[error("Missing 'schema' field")]
    MissingSchemaField,
    #[error("Missing 'properties' field")]
    MissingPropertiesField,
}

pub fn assemble(template: &Template, info_with_contents: &HashMap<Info, Content>) -> HashMap<String, Prompt> {
    info_with_contents.iter().map(|(info, content)| {
        let character_prompts = assemble_character_prompt(template, content);
        let stage_prompts = assemble_stage_prompt(content);

        (info.name.clone(), Prompt {
            character_prompts,
            stage_prompts,
        })
    }).collect()
}

fn assemble_character_prompt(template: &Template, content: &Content) -> HashMap<String, String> {
    let tcp = &template.character_prompts;  // template_character_prompts
    let ccp = &content.character_prompts; // content_character_prompts

    content.character_prompts.character_names.iter().map(|character_name| {
        let character_prompt_xml = [
            (&tcp.task_description,     &ccp.task_description),
            // (&tcp.input_description,    &ccp.input_description),
            // (&tcp.output_description,   &ccp.output_description),
            (&tcp.principle,            &ccp.principle),
            (&tcp.how_to_think,         &ccp.how_to_think),
            (&tcp.examples,             &ccp.examples),
        ].iter()
        .filter_map(|(template_field, content_field)| {
            content_field
                .get(character_name)
                .filter(|value| !value.is_empty()) // 先看 character_name 是否有值且不为空
                .or_else(|| content_field.get("assistant")) // 如果为空，则尝试获取 "assistant"
                .filter(|value| !value.is_empty()) // 仍然要检查是否为空
                .map(|value|
                    build_element(
                        template_field.element_name.clone(),
                        template_field.description.clone(),
                        value.clone(),
                    )
                )
        })
        .chain(std::iter::once({
            let stage_content = content.stage_prompt.iter()
                .map(|stage_prompt| format!("{}: {}\n", stage_prompt.name, stage_prompt.description))
                .collect::<String>();

            build_element(
                template.character_prompts.stage_description.element_name.clone(),
                template.character_prompts.stage_description.description.clone(),
                stage_content,
            )
        }))
        .collect::<String>();

        (character_name.clone(), character_prompt_xml)
    }).collect()
}


fn build_element(element_name: String, element_description: String, content: String) -> String {
    if content.is_empty() {
        String::new()
    } else {
        format!(
            "<{}>\n    <!-- {} -->\n{}</{}>\n",
            element_name, element_description, content, element_name
        )
    }
}

fn assemble_stage_prompt(content: &Content) -> HashMap<String, String>{
    content.stage_prompt.iter().map(|stage_prompt| {
        (stage_prompt.name.clone(), stage_prompt.content.clone())
    }).collect()
}


pub fn assemble_output_description(
    json_schema: serde_json::Value,
) -> error_stack::Result<String, OutputDescriptionError> {
    let json_schema = json_schema
        .get("json_schema")
        .ok_or(Report::new(OutputDescriptionError::MissingJsonSchemaField))?;

    let name = json_schema
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(OutputDescriptionError::MissingNameField))?;

    let description = json_schema
        .get("description")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(OutputDescriptionError::MissingDescriptionField))?;

    let schema = json_schema
        .get("schema")
        .ok_or(Report::new(OutputDescriptionError::MissingSchemaField))?;
    let properties = schema
        .get("properties")
        .ok_or(Report::new(OutputDescriptionError::MissingPropertiesField))?;

    let mut result = format!("你的回答需要包含以下内容。\n{}: {}\n", name, description);
    result.push_str(&extract_properties(properties, 1));

    Ok(result)
}


pub fn assemble_tools_prompt(json_schema_vec: Vec<serde_json::Value>) -> error_stack::Result<String, ChatToolSchemaError> {
    let mut tools = String::new();

    for json_schema in json_schema_vec {
        tools.push_str(
            &assemble_tool_prompt(json_schema)
                .change_context(ChatToolSchemaError::AssembleToolPrompt)?
                .as_str(),
        );
    }

    let indented_tools = tools
        .lines()
        .map(|line| format!("        {}", line)) // 先去掉前导空格，再加 8 空格
        .collect::<Vec<_>>()
        .join("\n");

    let result = format!(
        indoc! {"
            <ToolUse>
                当你需要调用某个工具时，请在回答中使用 <ToolUse></ToolUse> 标签，遵循以下要求：
                1. 每个标签仅包含一个工具调用，且工具的调用必须按照参数要求提供完整信息。
                2. 每个标签内的内容应包含：
                  - 工具名称：如 send_email。
                  - 工具描述：简要描述该工具的功能。
                  - 参数：提供工具所需的所有参数，并确保格式正确（如类型、命名等）。
                3. 你可以在同一回答中使用多个<FunctionCalling></FunctionCalling>标签，每个标签对应任意你想要的工具调用。
                4. 我会根据你提供的调用信息执行相应的操作，并将结果返回给你。
                5. 不要在回答中仅包含<FunctionCalling></FunctionCalling>标签, 带有一些其他的文字, 可以是你的想法或是其他想表述的内容。\n
                你可以使用以下工具：\n\n{}\n
            </ToolUse>
        "},
        indented_tools // 统一缩进后的工具描述
    );

    Ok(result)
}

fn assemble_tool_prompt(json_schema: serde_json::Value) -> error_stack::Result<String, ChatToolSchemaError> {
    // 提取 function 对象
    let function = json_schema.get("function")
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionField))?;
    let function_name = function.get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionName))?;
    let function_desc = function.get("description")
        .and_then(serde_json::Value::as_str)
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionDescription))?;

    // 提取 parameters 对象
    let parameters = function.get("parameters")
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionParameters))?;

    // 提取 parameters 下的 properties 字段
    let properties = parameters.get("properties")
        .ok_or(Report::new(ChatToolSchemaError::MissingFunctionProperties))?;

    // 构造最终的提示词字符串
    let mut result = String::new();
    result.push_str(&format!("函数名: {}\n函数描述: {}\n", function_name, function_desc));

    // 提取和展示各个参数的详细信息
    result.push_str(&extract_properties(properties, 1)); // 假设 extract_properties 内部没有错误，或者你会单独处理

    Ok(result)
}

pub fn extract_properties(properties: &serde_json::Value, indent: usize) -> String {
    let mut result = String::new();

    if let Some(props) = properties.as_object() {
        for (prop_name, prop_value) in props {
            if prop_name == "cot" {
                continue;
            }
            let mut line = format!("{}{}", "  ".repeat(indent), prop_name);

            // 提取 type
            if let Some(prop_type) = prop_value.get("type") {
                match prop_type {
                    serde_json::Value::String(type_str) => {
                        line.push_str(&format!(" ({})", type_str));
                    }
                    serde_json::Value::Array(type_array) => {
                        let types: Vec<String> = type_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        if !types.is_empty() {
                            line.push_str(&format!(" ([{}])", types.join(", ")));
                        }
                    }
                    _ => {}
                }
            }

            // 提取 description
            if let Some(prop_desc) = prop_value.get("description").and_then(|d| d.as_str()) {
                line.push_str(&format!(": {}", prop_desc));
            }

            // 提取 enum
            if let Some(prop_enum) = prop_value.get("enum") {
                if let Some(enum_values) = prop_enum.as_array() {
                    let enum_strings: Vec<String> = enum_values
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    if !enum_strings.is_empty() {
                        line.push_str(&format!(" (Enum: [{}])", enum_strings.join(", ")));
                    }
                }
            }

            result.push_str(&format!("{}\n", line));

            // 递归处理嵌套对象
            if prop_value.get("type") == Some(&serde_json::Value::String("object".to_string())) {
                if let Some(sub_properties) = prop_value.get("properties") {
                    result.push_str(&extract_properties(sub_properties, indent + 1));
                }
            }
        }
    }

    result
}
