use serde::{Deserialize, Serialize};

pub const META_MODEL_JSON: &str = include_str!("meta_model_317.json");

mod grammar;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LSPSpecMetaModel {
    /// Metadata about the spec version
    pub meta_data: MetaData,
    /// All LSP requests
    pub requests: Vec<Request>,
    /// All LSP notifications
    pub notifications: Vec<Notification>,
    /// All LSP structures
    pub structures: Vec<Structure>,
    /// All LSP enumerations
    #[serde(default)]
    pub enumerations: Vec<Enumeration>,
    /// All LSP type aliases
    #[serde(default)]
    pub type_aliases: Vec<TypeAlias>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaData {
    /// The version of the specification
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// The request method
    pub method: String,
    /// The message direction
    pub message_direction: MessageDirection,
    /// The request parameters
    pub params: Option<DataType>,
    /// The request result
    pub result: DataType,
    /// Optional partial result type
    #[serde(default)]
    pub partial_result: Option<DataType>,
    /// Optional error data type
    #[serde(default)]
    pub error_data: Option<DataType>,
    /// Optional registration method
    #[serde(default)]
    pub registration_method: Option<String>,
    /// Optional registration options
    #[serde(default)]
    pub registration_options: Option<DataType>,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the request is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    /// The notification method
    pub method: String,
    /// The message direction
    pub message_direction: MessageDirection,
    /// The notification parameters
    pub params: Option<DataType>,
    /// Optional registration method
    #[serde(default)]
    pub registration_method: Option<String>,
    /// Optional registration options
    #[serde(default)]
    pub registration_options: Option<DataType>,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the notification is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Structure {
    /// The structure name
    pub name: String,
    /// The structure's properties
    #[serde(default)]
    pub properties: Vec<Property>,
    /// Types this extends
    #[serde(default)]
    pub extends: Vec<DataType>,
    /// Types this mixes in
    #[serde(default)]
    pub mixins: Vec<DataType>,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the structure is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    /// The property name
    pub name: String,
    /// The property type
    #[serde(rename = "type")]
    pub data_type: DataType,
    /// Whether the property is optional
    #[serde(default)]
    pub optional: bool,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the property is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enumeration {
    /// The enumeration name
    pub name: String,
    /// The type of the enumeration
    #[serde(rename = "type")]
    pub data_type: DataType,
    /// The enumeration values
    pub values: Vec<EnumerationEntry>,
    /// Whether custom values are supported
    #[serde(default)]
    pub supports_custom_values: bool,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the enumeration is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumerationEntry {
    /// The entry name
    pub name: String,
    /// The entry value
    pub value: serde_json::Value,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the entry is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeAlias {
    /// The alias name
    pub name: String,
    /// The type being aliased
    #[serde(rename = "type")]
    pub type_def: DataType,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the alias is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageDirection {
    ClientToServer,
    ServerToClient,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum BaseType {
    #[serde(rename = "URI")]
    URI,
    #[serde(rename = "DocumentUri")]
    DocumentUri,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "uinteger")]
    UInteger,
    #[serde(rename = "decimal")]
    Decimal,
    #[serde(rename = "RegExp")]
    RegExp,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "null")]
    Null,
}

impl BaseType {
    pub const fn name(&self) -> &'static str {
        match self {
            BaseType::URI => "URI",
            BaseType::DocumentUri => "DocumentUri",
            BaseType::Integer => "Integer",
            BaseType::UInteger => "Uinteger",
            BaseType::Decimal => "Decimal",
            BaseType::RegExp => "RegExp",
            BaseType::String => "String",
            BaseType::Boolean => "Boolean",
            BaseType::Null => "Null",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DataType {
    Base(BaseType),
    Reference {
        name: String,
    },
    Array {
        element: Box<DataType>,
    },
    Or {
        items: Vec<DataType>,
    },
    And {
        items: Vec<DataType>,
    },
    Map {
        key: Box<DataType>,
        value: Box<DataType>,
    },
    Tuple {
        items: Vec<DataType>,
    },
    StringLiteral {
        value: String,
    },
    IntegerLiteral {
        value: i64,
    },
    BooleanLiteral {
        value: bool,
    },
    #[serde(rename = "literal")]
    StructureLiteral {
        value: StructureLiteral,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureLiteral {
    /// The structure's properties
    #[serde(default)]
    pub properties: Vec<Property>,
    /// Documentation
    #[serde(default)]
    pub documentation: Option<String>,
    /// Whether the structure is deprecated
    #[serde(default)]
    pub deprecated: Option<String>,
    /// When this was introduced
    #[serde(default)]
    pub since: Option<String>,
    /// Whether this is proposed
    #[serde(default)]
    pub proposed: bool,
}

#[test]
fn load_specs() {
    let meta_model: LSPSpecMetaModel =
        serde_json::from_str(META_MODEL_JSON).expect("Fail to serialize");
    eprintln!("{meta_model:?}");
}
