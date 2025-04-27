use lsp_types::{Position, TextDocumentIdentifier};

pub trait CodeContext {
    fn document(&self) -> Option<&TextDocumentIdentifier>;
    fn position(&self) -> Option<&Position>;
    fn range(&self) -> Option<&lsp_types::Range>;
}
