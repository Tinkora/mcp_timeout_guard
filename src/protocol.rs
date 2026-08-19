use serde_json::{Value, json};

pub const TIMEOUT_ERROR_CODE: i64 = -32001;
pub const CHILD_EXIT_ERROR_CODE: i64 = -32002;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestId(String);

impl RequestId {
    pub fn parse(value: &Value) -> Option<Self> {
        if value.is_null() || !(value.is_string() || value.is_number()) {
            return None;
        }
        serde_json::to_string(value).ok().map(Self)
    }

    pub fn value(&self) -> Value {
        serde_json::from_str(&self.0).expect("RequestId stores valid JSON")
    }
}

pub fn request_id(frame: &Value) -> Option<RequestId> {
    frame.get("id").and_then(RequestId::parse)
}

pub fn timeout_error(id: &RequestId, timeout_ms: u64) -> Value {
    error_response(
        id,
        TIMEOUT_ERROR_CODE,
        format!("MCP request timed out after {timeout_ms} ms"),
    )
}

pub fn child_exit_error(id: &RequestId) -> Value {
    error_response(
        id,
        CHILD_EXIT_ERROR_CODE,
        "MCP server exited before responding".to_string(),
    )
}

fn error_response(id: &RequestId, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.value(),
        "error": {
            "code": code,
            "message": message,
        },
    })
}

pub fn parse_client_frame(frame: &[u8]) -> Result<Value, String> {
    let text = std::str::from_utf8(frame).map_err(|_| "frame is not UTF-8".to_string())?;
    let value: Value =
        serde_json::from_str(text).map_err(|_| "frame is not valid JSON".to_string())?;
    if !value.is_object() {
        return Err("frame must be a JSON object".to_string());
    }
    if value.get("jsonrpc") != Some(&Value::String("2.0".to_string())) {
        return Err("frame must declare jsonrpc 2.0".to_string());
    }
    Ok(value)
}

pub fn write_frame(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).expect("JSON-RPC envelope is serializable");
    bytes.push(b'\n');
    bytes
}

pub fn response_id(frame: &Value) -> Option<RequestId> {
    request_id(frame)
}

pub fn is_request_with_id(frame: &Value) -> bool {
    request_id(frame).is_some()
}
