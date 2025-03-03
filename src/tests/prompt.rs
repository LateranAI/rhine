use crate::tests::format_test_block;
use crate::schema::json_schema::JsonSchema;
use rhine_schema_derive::{tool_schema_derive, JsonSchema};
use serde::Deserialize;
use crate::prompt::assembler::{assemble_output_description, assemble_tools_prompt};
use crate::schema::tool_schema::get_tool_function;

pub async fn test_prompt() {
    test_json_schema().await;
    test_tool_registry().await;
    test_assemble_output_discription().await;
    test_tool_schema().await;
    test_assemble_tools_prompt().await;
}

async fn test_json_schema() {
    let json_schema = StudentInfo::json_schema();
    format_test_block("StudentInfo::json_schema", || {
        serde_json::to_string_pretty(&json_schema).unwrap()
    });
    // assert_eq!(schema, expected);
}

async fn test_tool_schema() {
    // 调用生成的工具 schema 函数（名称自动生成为 send_email_tool_schema）
    let tool_schema = send_email_tool_schema();
    format_test_block("send_tool_schema", || {
        serde_json::to_string_pretty(&tool_schema).unwrap()
    });
}

async fn test_tool_registry() {
    // let tool_schema = send_email_tool_schema();
    format_test_block("tool_registry", || match get_tool_function("send_email") {
        Some(_) => "Function found".to_string(),
        None => "Function not found".to_string(),
    });
}

async fn test_assemble_output_discription() {
    let schema = StudentInfo::json_schema();
    let output_description = assemble_output_description(schema.clone()).unwrap();
    format_test_block("assemble_output_description", || output_description.clone());
    // assert_eq!(output_description, expected);
}

async fn test_assemble_tools_prompt() {
    let tool_schema = send_email_tool_schema();
    format_test_block("assemble_tools_prompt", || {
        assemble_tools_prompt(vec![tool_schema.clone(), tool_schema]).unwrap()
    });
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schema(name = "student_info", description = "用于记录学生信息", strict = true)]
pub struct StudentInfo {
    #[schema(desc = "生成学生信息时的思考过程", required = true)]
    cot: String,

    #[schema(desc = "学生的姓名", required = true)]
    name: String,

    #[schema(desc = "学生的年龄", required = true)]
    age: i32,

    #[schema(
        desc = "学生的年级",
        enum = "freshman, sophomore, junior, senior",
        required = true
    )]
    grade: Option<String>,

    #[schema(desc = "是否参加考试")]
    had_exam: bool,
}

#[derive(Deserialize, JsonSchema)]
#[schema(
    name = "SendEmailParams",
    description = "Parameters for sending email",
    inner = true,
    strict = true
)]
pub struct SendEmailParameters {
    #[schema(desc = "The recipient email address.")]
    pub to: String,
    #[schema(desc = "Email subject line.")]
    pub subject: String,
    #[schema(desc = "Body of the email message.")]
    pub body: String,
}

#[tool_schema_derive(
    description = "Send an email to a given recipient with a subject and message.",
    parameters = "SendEmailParameters",
    module_path = crate::tests::prompt,
    strict = true
)]
pub fn send_email(params: SendEmailParameters) {
    println!(
        "To: {} Subject: {} Body: {}",
        params.to, params.subject, params.body
    );
}
