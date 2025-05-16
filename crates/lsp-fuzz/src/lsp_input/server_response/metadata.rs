use std::collections::HashSet;

use libafl_bolts::SerdeAny;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, SerdeAny)]
pub struct LspResponseInfo {
    pub diagnostics: HashSet<Diagnostic>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, SerdeAny)]
pub struct Diagnostic {
    pub uri: String,
    pub range: lsp_types::Range,
}
