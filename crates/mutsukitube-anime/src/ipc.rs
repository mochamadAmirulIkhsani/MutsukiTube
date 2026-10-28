use crate::models::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_IPC_LINE_BYTES: usize = 1 << 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcRequest {
    pub protocol_version: u32,
    pub request_id: String,
    pub action: String,
    #[serde(default = "default_params")]
    pub params: Value,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "deadlineMs"
    )]
    pub deadline_ms: Option<u64>,
}

fn default_params() -> Value {
    Value::Object(serde_json::Map::new())
}

impl IpcRequest {
    pub fn new(request_id: impl Into<String>, action: impl Into<String>, params: Value) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            action: action.into(),
            params,
            deadline_ms: None,
        }
    }

    pub fn with_deadline(mut self, deadline_ms: u64) -> Self {
        self.deadline_ms = Some(deadline_ms);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcResponse {
    pub protocol_version: u32,
    pub request_id: String,
    pub success: bool,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub error: Option<IpcErrorBody>,
}

impl IpcResponse {
    pub fn data_field(&self, name: &str) -> Option<Value> {
        self.data.as_ref()?.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_with_expected_fields() {
        let req =
            IpcRequest::new("req-1", "search", json!({"query": "Naruto"})).with_deadline(30000);
        let value: Value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["protocolVersion"], 1);
        assert_eq!(value["requestId"], "req-1");
        assert_eq!(value["action"], "search");
        assert_eq!(value["params"]["query"], "Naruto");
        assert_eq!(value["deadlineMs"], 30000);
    }

    #[test]
    fn response_deserializes_success() {
        let raw =
            r#"{"protocolVersion":1,"requestId":"req-1","success":true,"data":{"results":[]}}"#;
        let resp: IpcResponse = serde_json::from_str(raw).unwrap();
        assert!(resp.success);
        assert_eq!(resp.request_id, "req-1");
        assert_eq!(resp.data_field("results").unwrap(), json!([]));
    }

    #[test]
    fn response_deserializes_structured_error() {
        let raw = r#"{"protocolVersion":1,"requestId":"req-2","success":false,"error":{"code":"PARSER_CHANGED","message":"layout changed","retryable":false}}"#;
        let resp: IpcResponse = serde_json::from_str(raw).unwrap();
        assert!(!resp.success);
        let err = resp.error.unwrap();
        assert_eq!(err.code, "PARSER_CHANGED");
        assert!(!err.retryable);
    }

    #[test]
    fn info_response_shape_parses() {
        let raw = r#"{"protocolVersion":1,"requestId":"i","success":true,"data":{"info":{"id":"ylnime","name":"YLnime","pluginVersion":"1.0.0","protocolVersion":1,"baseUrl":"https://ylnime.com","language":"Indonesian","description":"test"}}}"#;
        let resp: IpcResponse = serde_json::from_str(raw).unwrap();
        let info: crate::models::PluginInfo =
            serde_json::from_value(resp.data_field("info").unwrap()).unwrap();
        assert_eq!(info.id, "ylnime");
        assert_eq!(info.protocol_version, 1);
    }
}
