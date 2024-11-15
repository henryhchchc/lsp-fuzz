use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize};
use static_assertions::const_assert_eq;

/// JSON-RPC 2.0 protocol version.
#[derive(Debug, Clone, Copy)]
pub struct JsonRPC20;

impl JsonRPC20 {
    /// The string representation of the JSON-RPC 2.0 protocol version.
    pub const VERSION: &'static str = "2.0";
}

// Ensure that `JsonRPC20` is a ZST.
const_assert_eq!(std::mem::size_of::<JsonRPC20>(), 0);

impl Serialize for JsonRPC20 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(Self::VERSION)
    }
}

impl<'de> Deserialize<'de> for JsonRPC20 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{Error, Unexpected};
        let version: &str = Deserialize::deserialize(deserializer)?;
        if version == Self::VERSION {
            Ok(Self)
        } else {
            Err(Error::invalid_value(
                Unexpected::Str(version),
                &Self::VERSION,
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRPCMessage {
    jsonrpc: JsonRPC20,
    pub id: usize,
    pub method: Cow<'static, str>,
    pub params: serde_json::Value,
}

impl JsonRPCMessage {
    pub const CONTENT_LENGTH_HEADER: &'static [u8] = b"Content-Length: ";
    pub const HEADER_BODY_SEP: &'static [u8] = b"\r\n\r\n";

    pub const fn new(id: usize, method: Cow<'static, str>, params: serde_json::Value) -> Self {
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
        let content_length = content.len().to_string().into_bytes();
        Self::CONTENT_LENGTH_HEADER
            .iter()
            .copied()
            .chain(content_length)
            .chain(Self::HEADER_BODY_SEP.iter().copied())
            .chain(content)
            .collect()
    }
}

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
    use crate::lsp::Message;
    use lsp_types::{
        request::{Initialize, Request},
        InitializeParams, WorkspaceFolder,
    };

    let request = Message::Initialize(InitializeParams {
        workspace_folders: Some(vec![WorkspaceFolder {
            uri: "file:///path/to/folder".parse().unwrap(),
            name: "folder".to_string(),
        }]),
        ..Default::default()
    });
    let (method, params) = request.as_json();
    let jsonrpc = JsonRPCMessage::new(1, method.into(), params).to_lsp_payload();
    let header = b"Content-Length: 177\r\n\r\n";
    assert_eq!(jsonrpc[..header.len()], header[..]);
    let json_value: serde_json::Value = serde_json::from_slice(&jsonrpc[header.len()..]).unwrap();
    assert_eq!(json_value["jsonrpc"], JsonRPC20::VERSION);
    assert_eq!(json_value["id"], 1);
    assert_eq!(json_value["method"], Initialize::METHOD);
    assert!(json_value["params"]["workspaceFolders"][0]["uri"]
        .as_str()
        .unwrap()
        .contains("path/to/folder"));
}
