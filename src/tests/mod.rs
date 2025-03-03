use tracing::log::info;
use crate::tests::chat::test_chat;
use crate::tests::prompt::test_prompt;

mod chat;
mod prompt;

#[tokio::test]
pub async fn test() {
    // 初始化日志配置
    let _guard = clia_tracing_config::build()
        .filter_level("info")
        .with_ansi(true)
        .to_stdout(true)
        .directory("./logs")
        .file_name("test.log")
        .init();
    println!("log level: {}", "info");
    test_prompt().await;
    // test_chat().await;
}

pub fn format_test_block<F>(title: &str, content_fn: F)
where
    F: FnOnce() -> String,
{
    info!(
        "\n\
        [TEST]>>>>>>>>>>>>>>>>>>>>>>>>>>>\n\
        [{}]\n{}\n\
        --------------------------------------------------",
        title,
        content_fn()
    );
}