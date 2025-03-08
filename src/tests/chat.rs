use crate::chat::chat_single::SingleChat;
use crate::config::Config;
use crate::config::ModelCapability::{Think, ToolUse};
use crate::schema::json_schema::JsonSchema;
use crate::tests::format_test_block;
use rhine_schema_derive::{JsonSchema, tool_schema_derive};
use serde::Deserialize;

pub async fn test_chat() {
    Config::add_api_source(
        "pumpkin",
        "https://api.pumpkinaigc.online/v1/chat/completions",
        20,
    );
    Config::add_api_info(
        "pumpkin-ds-r1",
        "deepseek-r1",
        Think,
        "pumpkin",
        "sk-cPdegaWl8YFcKZYs8a108b5f741844D9A1E0B90e724bBe23",
    );
    Config::add_api_info(
        "pumpkin-gpt-4o",
        "gpt-4o",
        ToolUse,
        "pumpkin",
        "sk-cPdegaWl8YFcKZYs8a108b5f741844D9A1E0B90e724bBe23",
    );

    test_single_chat().await;
    // test_single_chat_get_json().await;
    // test_single_chat_get_tool().await;
}

async fn test_single_chat() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-gpt-4o", "", true);

    let answer_1 = chat.get_resp("深度思考strawberry有几个r").await.unwrap();
    let message_1 = chat.base.session.clone();
    format_test_block("chat_single_round", || {
        format!("answer: {}, message: {:?}", answer_1, message_1)
    });

    let answer_2 = chat.get_resp("你确定吗?").await.unwrap();
    let message_2 = chat.base.session.clone();
    format_test_block("chat_multi_round", || {
        format!("answer_2: {}\nmessage_2: {:?}\n", answer_2, message_2)
    });

    let answer_3 = chat.get_resp_again([0].as_ref()).await.unwrap();
    let message_3 = chat.base.session.clone();
    format_test_block("chat_answer_again", || {
        format!("answer_3: {}\nmessage_3: {:?}\n", answer_3, message_3)
    });

    let answer_4 = chat
        .get_resp_with_new_question(
            [].as_ref(),
            "straw中有一个r, berry中有两个r, 深度思考strawberry有几个r?",
        )
        .await
        .unwrap();
    let message_4 = chat.base.session.clone();
    format_test_block("chat_new_question", || {
        format!("answer_4: {}\nmessage_4: {:?}", answer_4, message_4)
    });
}

async fn test_single_chat_get_json() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-ds-r1", "", true);
    let answer = chat
        .get_json_answer::<StudentInfo>("编造一个学生信息")
        .await
        .unwrap();
    format_test_block("structured_answer", || format!("StudentInfo: {:?}", answer));
}

async fn test_single_chat_get_tool() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-ds-r1", "", true);
    chat.set_tools(vec![send_email_tool_schema()]);
    let answer = chat
        .get_tool_answer("随意编造信息发送一封邮件")
        .await
        .unwrap();
    format_test_block("structured_answer", || {
        format!(
            "ToolUseResult:\n Answer:{}\nResponse: {:?}",
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
    module_path = crate::tests::chat,
    strict = true
)]
pub fn send_email(params: SendEmailParameters) {
    println!(
        "To: {} Subject: {} Body: {}",
        params.to, params.subject, params.body
    );
}
