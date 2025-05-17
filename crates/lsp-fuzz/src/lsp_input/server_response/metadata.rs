use std::{collections::HashSet, hash::Hash};

use derive_new::new as New;
use libafl_bolts::SerdeAny;
use lsp_types::{
    CallHierarchyItem, CodeAction, CodeLens, Command, CompletionItem, DocumentLink, InlayHint,
    TypeHierarchyItem, WorkspaceSymbol,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, SerdeAny)]
pub struct LspResponseInfo {
    pub diagnostics: HashSet<Diagnostic>,
    pub param_fragments: ParamFragments,
    pub symbol_ranges: HashSet<SymbolRange>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, SerdeAny)]
pub struct Diagnostic {
    pub uri: lsp_types::Uri,
    pub range: lsp_types::Range,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, SerdeAny)]
pub struct ParamFragments {
    pub code_actions: HashSet<CodeAction>,
    pub commands: HashSet<Command>,
    pub inlay_hints: HashSet<InlayHint>,
    pub completion_items: HashSet<CompletionItem>,
    pub code_lens: HashSet<CodeLens>,
    pub workspace_symbols: HashSet<WorkspaceSymbol>,
    pub type_hierarchy_items: HashSet<TypeHierarchyItem>,
    pub call_hierarchy_items: HashSet<CallHierarchyItem>,
    pub document_links: HashSet<DocumentLink>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, SerdeAny, New)]
pub struct SymbolRange {
    pub uri: lsp_types::Uri,
    pub range: lsp_types::Range,
}

pub trait ContainsFragment<T> {
    fn fragments(&self) -> &HashSet<T>;
}

macro_rules! contains_items {
    ($field: ident: $type:ty) => {
        impl ContainsFragment<$type> for ParamFragments {
            fn fragments(&self) -> &HashSet<$type> {
                &self.$field
            }
        }
    };
}

contains_items!(code_actions: CodeAction);
contains_items!(commands: Command);
contains_items!(inlay_hints: InlayHint);
contains_items!(completion_items: CompletionItem);
contains_items!(code_lens: CodeLens);
contains_items!(workspace_symbols: WorkspaceSymbol);
contains_items!(type_hierarchy_items: TypeHierarchyItem);
contains_items!(call_hierarchy_items: CallHierarchyItem);
contains_items!(document_links: DocumentLink);
