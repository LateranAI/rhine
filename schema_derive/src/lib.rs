// src/lib.rs

// 本 crate 提供两大功能：
// 1. 为结构体生成 JSON Schema（用于数据验证等），使用 #[derive(JsonSchema)] 与 #[schema(...)] 属性。
// 2. 为工具函数生成用于 function calling 的工具 schema，使用属性宏 #[function_tool(...)]。
//
// 示例用法：
//
// ```rust
// // 参数结构体，生成 JSON Schema（需实现 JsonSchema）
// #[derive(JsonSchema)]
// #[schema(name = "SendEmailParams", description = "Parameters for sending email", strict = true)]
// pub struct SendEmailParameters {
//     #[schema(desc = "The recipient email address.")]
//     pub to: String,
//     #[schema(desc = "Email subject line.")]
//     pub subject: String,
//     #[schema(desc = "Body of the email message.")]
//     pub body: String,
// }
//
// // 工具函数，使用 #[function_tool(...)] 标记，指定工具描述、参数类型、严格模式等。
// #[function_tool(
//     description = "Send an email to a given recipient with a subject and message.",
//     parameters = "SendEmailParameters",
//     strict = true
// )]
// pub fn send_email(params: SendEmailParameters) {
//     // 实际邮件发送逻辑...
// }
//
// // 调用生成的工具 schema 函数（名称自动生成为 send_email_tool_schema）
// let tool_schema = send_email_tool_schema();
// println!("{}", tool_schema.to_string());
// ```

use proc_macro::TokenStream;

mod attributes;
mod generator;
mod type_helpers;
mod tools;
mod path_solver;

#[proc_macro_derive(JsonSchema, attributes(schema))]
pub fn json_schema_derive(input: TokenStream) -> TokenStream {
    generator::json_schema_derive_impl(input)
}

#[proc_macro_attribute]
pub fn tool_schema_derive(attr: TokenStream, item: TokenStream) -> TokenStream {
    tools::function_tool_attr_impl(attr, item)
}