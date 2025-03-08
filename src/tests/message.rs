use crate::tests::format_test_block;
use crate::chat::message::{Messages, Role};

pub async fn test_message() {
    test_message_creation();
    test_add_message();
    test_get_node_by_path();
    test_update_content();
    test_delete_message();
    test_to_api_format();
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
    let api_format = msg.to_api_format(&Role::Assistant);

    assert_eq!(api_format.get("role").unwrap(), "user");
    assert_eq!(api_format.get("content").unwrap(), "Hello");

    let character_msg = Messages::new(Role::Character("Alice".to_string()), "Hi Bob".to_string());

    // 当角色不是当前发言者
    // When the role is not the current speaker
    let api_format = character_msg.to_api_format(&Role::Assistant);
    assert_eq!(api_format.get("role").unwrap(), "user");
    assert_eq!(api_format.get("content").unwrap(), "Alice said: Hi Bob");

    // 当角色是当前发言者
    // When the role is the current speaker
    let api_format = character_msg.to_api_format(&Role::Character("Alice".to_string()));
    assert_eq!(api_format.get("role").unwrap(), "assistant");
    assert_eq!(api_format.get("content").unwrap(), "Hi Bob");

    format_test_block("to_api_format", || format!("{:?}", api_format))
}