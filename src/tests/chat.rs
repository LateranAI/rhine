use crate::chat::chat_single::SingleChat;
use crate::config::Config;
use crate::config::ModelCapability::{Think, ToolUse};
use crate::schema::json_schema::JsonSchema;
use crate::tests::format_test_block;
use rhine_schema_derive::{JsonSchema, tool_schema_derive};
use serde::Deserialize;
use crate::chat::message::{Messages, Role};

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

    // test_message_creation();
    // test_add_message();
    // test_get_node_by_path();
    // test_update_content();
    // test_delete_message();
    // test_to_api_format();

    test_single_chat().await;
    test_single_chat_get_json().await;
    test_single_chat_get_tool().await;
}

fn test_message_creation() {
    let msg = Messages::new(Role::User, "Hello".to_string());
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.path.len(), 0);
    assert_eq!(msg.child.len(), 0);
    format_test_block("message_creation", || format!("{:?}", msg))
}

fn test_add_message() {
    let mut root = Messages::new(Role::System, "System prompt".to_string());

    // 添加第一级消息
    // Add first level message
    root.add(&[], Role::User, "User message".to_string())
        .unwrap();
    assert_eq!(root.child.len(), 1);
    assert_eq!(root.child[0].role, Role::User);
    assert_eq!(root.child[0].content, "User message");
    assert_eq!(root.child[0].path, vec![0]);

    // 添加第二级消息
    // Add second level message
    root.add(&[0], Role::Assistant, "Assistant response".to_string())
        .unwrap();
    assert_eq!(root.child[0].child.len(), 1);
    assert_eq!(root.child[0].child[0].role, Role::Assistant);
    assert_eq!(root.child[0].child[0].content, "Assistant response");
    assert_eq!(root.child[0].child[0].path, vec![0, 0]);
    format_test_block("add_message", || format!("{:?}", root))
}

fn test_get_node_by_path() {
    let mut root = Messages::new(Role::System, "System prompt".to_string());
    root.add(&[], Role::User, "User message".to_string())
        .unwrap();
    root.add(&[0], Role::Assistant, "Assistant response".to_string())
        .unwrap();

    let node = root.get_node_by_path(&[0, 0]).unwrap();
    assert_eq!(node.role, Role::Assistant);
    assert_eq!(node.content, "Assistant response");
    format_test_block("get_node_by_path", || format!("{:?}", node))
}

fn test_update_content() {
    let mut root = Messages::new(Role::System, "System prompt".to_string());
    root.add(&[], Role::User, "User message".to_string())
        .unwrap();

    root.update_content(&[0], "Updated user message".to_string())
        .unwrap();
    assert_eq!(root.child[0].content, "Updated user message");
    format_test_block("update_content", || format!("{:?}", root))
}

fn test_delete_message() {
    let mut root = Messages::new(Role::System, "System prompt".to_string());
    root.add(&[], Role::User, "User 1".to_string()).unwrap();
    root.add(&[], Role::User, "User 2".to_string()).unwrap();
    root.add(&[], Role::User, "User 3".to_string()).unwrap();

    // 删除第二条消息
    // Delete the second message
    root.delete(&[1]).unwrap();

    assert_eq!(root.child.len(), 2);
    assert_eq!(root.child[0].content, "User 1");
    assert_eq!(root.child[1].content, "User 3");
    assert_eq!(root.child[1].path, vec![1]);

    format_test_block("delete_message", || format!("{:?}", root))
}

fn test_to_api_format() {
    let msg = Messages::new(Role::User, "Hello".to_string());
    let api_format = msg.to_api_format_single(&Role::Assistant);

    assert_eq!(api_format.get("role").unwrap(), "user");
    assert_eq!(api_format.get("content").unwrap(), "Hello");

    let character_msg = Messages::new(Role::Character("Alice".to_string()), "Hi Bob".to_string());

    // 当角色不是当前发言者
    // When the role is not the current speaker
    let api_format = character_msg.to_api_format_single(&Role::Assistant);
    assert_eq!(api_format.get("role").unwrap(), "user");
    assert_eq!(api_format.get("content").unwrap(), "Alice said: Hi Bob");

    // 当角色是当前发言者
    // When the role is the current speaker
    let api_format = character_msg.to_api_format_single(&Role::Character("Alice".to_string()));
    assert_eq!(api_format.get("role").unwrap(), "assistant");
    assert_eq!(api_format.get("content").unwrap(), "Hi Bob");

    format_test_block("to_api_format", || format!("{:?}", api_format))
}

async fn test_single_chat() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-ds-r1", "", true);
    let answer = chat.get_answer("深度思考strawberry有几个r").await.unwrap();
    format_test_block("single_chat", || answer);
}

async fn test_single_chat_get_json() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-ds-r1", "", false);
    let answer = chat
        .get_json_answer::<StudentInfo>("编造一个学生信息")
        .await
        .unwrap();
    format_test_block("structured_answer", || format!("StudentInfo: {:?}", answer));
}

async fn test_single_chat_get_tool() {
    let mut chat = SingleChat::new_with_api_name("pumpkin-ds-r1", "", false);
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
