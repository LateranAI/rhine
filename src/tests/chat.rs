use crate::schema::json_schema::JsonSchema;
use crate::chat::chat_single::SingleChat;
use crate::config::Config;
use crate::config::ModelCapability::Think;
use crate::tests::format_test_block;
use futures::executor::block_on;
use schema_derive::{tool_schema_derive, JsonSchema};
use serde::Deserialize;

pub async fn test_chat() {
    Config::add_api_source("pumpkin", "https://api.pumpkinaigc.online/v1/chat/completions", 20);
    Config::add_api_info(
        "pumpkin-gpt-o3-mini",
        "o3-mini",
        Think,
        "pumpkin",
        "sk-cPdegaWl8YFcKZYs8a108b5f741844D9A1E0B90e724bBe23",
    );

    // test_single_chat().await;
    test_single_chat_get_json().await;
    // test_single_chat_get_tool().await;
}

async fn test_single_chat() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-gpt-o3-mini", "", false);
    let answer = chat.get_answer("深度思考strawberry有几个r").await.unwrap();
    format_test_block("single_chat", || answer);
}

async fn test_single_chat_get_json() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-gpt-o3-mini", "", false);
    format_test_block("structured_answer", || {
        format!(
            "StudentInfo: {:?}",
            block_on(async {
                chat.get_json_answer::<StudentInfo>("编造一个学生信息")
                    .await
                    .unwrap()
            })
        )
    });
}

async fn test_single_chat_get_tool() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-gpt-o3-mini", "", false);
    chat.set_tools(vec![send_email_tool_schema()]);
    let answer = chat
        .get_tool_answer("随意编造信息发送一封邮件")
        .await
        .unwrap();
    format_test_block("structured_answer", || {
        format!(
            "FunctionCallingResult:\n Answer:{}\nResponse: {:?}",
            answer.0, answer.1
        )
    });
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schema(name = "student_info", description = "用于记录学生信息", strict = true)]
pub struct StudentInfo {
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
    params = true,
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
    module_path = crate::tests::chat,
    strict = true
)]
pub fn send_email(params: SendEmailParameters) {
    println!(
        "To: {} Subject: {} Body: {}",
        params.to, params.subject, params.body
    );
}
