use mcp_timeout_guard::{
    CHILD_EXIT_ERROR_CODE, RequestId, TIMEOUT_ERROR_CODE, child_exit_error, parse_client_frame,
    request_id, timeout_error, write_frame,
};
use serde_json::json;

#[test]
fn request_id_accepts_strings_and_numbers_but_not_notifications() {
    let request = json!({"jsonrpc":"2.0","id":"abc","method":"initialize"});
    assert_eq!(
        request_id(&request),
        Some(RequestId::parse(&json!("abc")).unwrap())
    );
    assert!(request_id(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());
    assert!(request_id(&json!({"jsonrpc":"2.0","id":null,"method":"notify"})).is_none());
}

#[test]
fn timeout_error_preserves_only_the_request_id() {
    let id = RequestId::parse(&json!(7)).unwrap();
    let value = timeout_error(&id, 1234);
    assert_eq!(value["id"], 7);
    assert_eq!(value["error"]["code"], TIMEOUT_ERROR_CODE);
    assert_eq!(
        value["error"]["message"],
        "MCP request timed out after 1234 ms"
    );
    assert!(value["error"].get("data").is_none());
}

#[test]
fn child_exit_error_does_not_echo_payload() {
    let id = RequestId::parse(&json!("secret-id-is-not-a-payload")).unwrap();
    let value = child_exit_error(&id);
    assert_eq!(value["error"]["code"], CHILD_EXIT_ERROR_CODE);
    assert!(
        !serde_json::to_string(&value)
            .unwrap()
            .contains("secret-input")
    );
}

#[test]
fn client_frame_requires_json_rpc_object() {
    let value = parse_client_frame(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).unwrap();
    assert_eq!(value["method"], "ping");
    assert!(parse_client_frame(br#"[]"#).is_err());
    assert!(parse_client_frame(br#"{"jsonrpc":"1.0"}"#).is_err());
}

#[test]
fn frames_are_newline_delimited() {
    let frame = write_frame(&json!({"jsonrpc":"2.0","id":1,"result":{}}));
    assert_eq!(frame.last(), Some(&b'\n'));
}
