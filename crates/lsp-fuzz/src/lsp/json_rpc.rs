use std::{
    borrow::Cow,
    fmt::{self, Display},
    io::{self, BufRead, Read},
};

use serde::{Deserialize, Deserializer, Serialize};
use static_assertions::const_assert_eq;

/// JSON-RPC 2.0 protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        // NOTE: We _must_ use a Cow here to handle both owned and borrowed strings.
        //       When reading from a reader, the value is owned.
        //       When reading from a slice, the value can be borrowed.
        let version: Cow<'_, str> = Deserialize::deserialize(deserializer)?;
        if version == Self::VERSION {
            Ok(Self)
        } else {
            Err(Error::invalid_value(
                Unexpected::Str(version.as_ref()),
                &Self::VERSION,
            ))
        }
    }
}

/// The ID of a JSON-RPC message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum MessageId {
    Number(usize),
    String(Cow<'static, str>),
}

impl From<usize> for MessageId {
    fn from(id: usize) -> Self {
        Self::Number(id)
    }
}

impl From<String> for MessageId {
    fn from(id: String) -> Self {
        Self::String(id.into())
    }
}

impl From<&'static str> for MessageId {
    fn from(id: &'static str) -> Self {
        Self::String(id.into())
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(id) => write!(f, "{}", id),
            Self::String(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRPCMessage {
    Request {
        jsonrpc: JsonRPC20,
        id: MessageId,
        method: Cow<'static, str>,
        params: serde_json::Value,
    },
    Notification {
        jsonrpc: JsonRPC20,
        method: Cow<'static, str>,
        params: serde_json::Value,
    },
    Response {
        jsonrpc: JsonRPC20,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<MessageId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<serde_json::Value>,
    },
}

const CONTENT_LENGTH_HEADER: &[u8] = b"Content-Length";
const HEADER_SEP: &[u8] = b": ";
const HEADER_BODY_SEP: &[u8] = b"\r\n\r\n";

impl JsonRPCMessage {
    pub fn request(
        id: impl Into<MessageId>,
        method: Cow<'static, str>,
        params: serde_json::Value,
    ) -> Self {
        Self::Request {
            jsonrpc: JsonRPC20,
            id: id.into(),
            method,
            params,
        }
    }

    pub const fn notification(method: Cow<'static, str>, params: serde_json::Value) -> Self {
        Self::Notification {
            jsonrpc: JsonRPC20,
            method,
            params,
        }
    }

    pub fn response(
        id: Option<impl Into<MessageId>>,
        result: Option<serde_json::Value>,
        error: Option<serde_json::Value>,
    ) -> Self {
        Self::Response {
            jsonrpc: JsonRPC20,
            id: id.map(Into::into),
            result,
            error,
        }
    }

    pub const fn id(&self) -> Option<&MessageId> {
        match self {
            Self::Request { id, .. } => Some(id),
            Self::Response { id, .. } => id.as_ref(),
            Self::Notification { .. } => None,
        }
    }

    pub const fn method(&self) -> Option<&Cow<'_, str>> {
        if let Self::Request { method, .. } | Self::Notification { method, .. } = self {
            Some(method)
        } else {
            None
        }
    }

    pub fn to_lsp_payload(&self) -> Vec<u8> {
        let content =
            serde_json::to_vec(self).expect("Serialization of serde_json::Value cannot fail.");
        let content_length = content.len().to_string().into_bytes();
        CONTENT_LENGTH_HEADER
            .iter()
            .copied()
            .chain(HEADER_SEP.iter().copied())
            .chain(content_length)
            .chain(HEADER_BODY_SEP.iter().copied())
            .chain(content)
            .collect()
    }
}

impl JsonRPCMessage {
    // It does not compile without `R: Read`.
    pub fn read_lsp_payload<R: Read + BufRead + ?Sized>(reader: &mut R) -> io::Result<Self> {
        use io::{Error, ErrorKind::InvalidData};
        let content_size = Self::read_headers(reader)?.ok_or(Error::new(
            InvalidData,
            "The message does not contain a length header",
        ))?;
        let rdr = reader.take(content_size as u64);
        let json: Self = serde_json::from_reader(rdr).map_err(|e| Error::new(InvalidData, e))?;
        Ok(json)
    }

    fn read_headers<R: BufRead + ?Sized>(reader: &mut R) -> io::Result<Option<usize>> {
        use io::{Error, ErrorKind::InvalidData};
        let mut content_length = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                return Err(Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Could not read any data",
                ));
            }
            let line = line.strip_suffix("\r\n").ok_or(Error::new(
                InvalidData,
                "The header does not end with \\r\\n",
            ))?;
            if line.is_empty() {
                return Ok(content_length);
            }
            let (key, value) = line
                .split_once(": ")
                .ok_or_else(|| Error::new(InvalidData, format!("Invalid header: {}", line)))?;
            if key == "Content-Length" {
                let value = value.parse().map_err(|e| Error::new(InvalidData, e))?;
                content_length = Some(value);
            }
        }
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

    let data = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"processId\":null,\"rootUri\":null,\"capabilities\":{}}}";
    let _: JsonRPCMessage = serde_json::from_slice(&data[..]).unwrap();
}

#[test]
fn test_lsp_request() {
    use lsp_types::{
        InitializeParams, WorkspaceFolder,
        request::{Initialize, Request},
    };

    use crate::lsp::ClientToServerMessage;

    let request = ClientToServerMessage::Initialize(InitializeParams {
        workspace_folders: Some(vec![WorkspaceFolder {
            uri: "lsp-fuzz://".parse().unwrap(),
            name: "folder".to_string(),
        }]),
        ..Default::default()
    });
    let mut id = 1;
    let jsonrpc = request
        .into_json_rpc(&mut id, Some("file:///path/to/folder/"))
        .to_lsp_payload();
    let header = b"Content-Length: 178\r\n\r\n";
    assert_eq!(jsonrpc[..header.len()], header[..]);
    let json_value: serde_json::Value = serde_json::from_slice(&jsonrpc[header.len()..]).unwrap();
    assert_eq!(json_value["jsonrpc"], JsonRPC20::VERSION);
    assert_eq!(json_value["id"], 1);
    assert_eq!(json_value["method"], Initialize::METHOD);
    assert!(
        json_value["params"]["workspaceFolders"][0]["uri"]
            .as_str()
            .unwrap()
            .contains("path/to/folder")
    );
}

#[test]
fn parse_payload() {
    let mut payload = b"Content-Length: 107\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"processId\":null,\"rootUri\":null,\"capabilities\":{}}}".as_slice();
    JsonRPCMessage::read_lsp_payload(&mut payload).unwrap();
}
