use serde::{Deserialize, Serialize};

mod message;
pub use message::Message;

#[derive(Debug, Clone, Copy)]
pub struct JsonRPC20;

impl Serialize for JsonRPC20 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("2.0")
    }
}

impl<'de> Deserialize<'de> for JsonRPC20 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let version: &str = Deserialize::deserialize(deserializer)?;
        if version == "2.0" {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("Invalid JSON-RPC version"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRPCMessage {
    jsonrpc: JsonRPC20,
    id: usize,
    method: String,
    params: serde_json::Value,
}

impl JsonRPCMessage {
    pub fn new(id: usize, message: &Message) -> Self {
        let (method, params) = message.as_json();
        let method = method.to_owned();
        Self {
            jsonrpc: JsonRPC20,
            id,
            method,
            params,
        }
    }

    pub fn to_lsp_payload(&self) -> Vec<u8> {
        let content =
            serde_json::to_vec(self).expect("Serialization of serde_json::Value cannot fail.");
        let content_length = content.len();
        let mut result = format!("Content-Length: {content_length}\r\n\r\n").into_bytes();
        result.extend(content);
        result
    }
}

#[cfg(test)]
mod test {
    use lsp_types::request::{Initialize, Request};

    use super::{JsonRPC20, JsonRPCMessage, Message};

    #[test]
    fn jsonrpc_version_serialize() {
        let jsonrpc = JsonRPC20;
        let serialized = serde_json::to_value(jsonrpc).unwrap();
        assert!(serialized.is_string());
        assert_eq!(serialized.as_str().unwrap(), "2.0");
    }

    #[test]
    fn jsonrpc_version_deserialize() {
        let jsonrpc: JsonRPC20 = serde_json::from_str("\"2.0\"").unwrap();
        assert!(matches!(jsonrpc, JsonRPC20));
    }

    #[test]
    fn test_lsp_request() {
        let request = Message::Initialize(lsp_types::InitializeParams {
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: "file:///path/to/folder".parse().unwrap(),
                name: "folder".to_string(),
            }]),
            ..Default::default()
        });
        let jsonrpc = JsonRPCMessage::new(1, &request).to_lsp_payload();
        let header = b"Content-Length: 177\r\n\r\n";
        assert_eq!(jsonrpc[..header.len()], header[..]);
        let json_value: serde_json::Value =
            serde_json::from_slice(&jsonrpc[header.len()..]).unwrap();
        assert_eq!(json_value["jsonrpc"], "2.0");
        assert_eq!(json_value["id"], 1);
        assert_eq!(json_value["method"], Initialize::METHOD);
        assert!(json_value["params"]["workspaceFolders"][0]["uri"]
            .as_str()
            .unwrap()
            .contains("path/to/folder"));
    }
}
