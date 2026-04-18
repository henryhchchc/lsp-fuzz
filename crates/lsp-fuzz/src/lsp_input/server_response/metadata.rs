use std::{collections::HashSet, hash::Hash};

use derive_new::new as New;
use libafl_bolts::SerdeAny;
use lsp_types::{
    CallHierarchyItem, CodeAction, CodeActionOrCommand, CodeLens, Command, CompletionItem,
    CompletionResponse, DocumentLink, InlayHint, OneOf, SymbolInformation, TypeHierarchyItem,
    WorkspaceSymbol,
};
use serde::{Deserialize, Serialize};

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, Serialize, Deserialize, SerdeAny)]
pub struct LspResponseInfo {
    pub diagnostics: HashSet<Diagnostic>,
    pub param_fragments: ParamFragments,
    pub symbol_ranges: HashSet<SymbolRange>,
}

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, SerdeAny)]
pub struct Diagnostic {
    pub uri: lsp_types::Uri,
    pub range: lsp_types::Range,
}

#[allow(clippy::unsafe_derive_deserialize)]
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

impl ParamFragments {
    pub fn collect_code_actions(
        &mut self,
        code_actions_or_commands: Option<Vec<CodeActionOrCommand>>,
    ) {
        if let Some(cas) = code_actions_or_commands {
            for ca in cas {
                match ca {
                    CodeActionOrCommand::Command(command) => {
                        self.commands.insert(command);
                    }
                    CodeActionOrCommand::CodeAction(code_action) => {
                        self.code_actions.insert(code_action);
                    }
                }
            }
        }
    }

    pub fn collect_inlay_hints(&mut self, inlay_hints: Option<Vec<InlayHint>>) {
        if let Some(inlay_hints) = inlay_hints {
            self.inlay_hints.extend(inlay_hints);
        }
    }

    pub fn collect_completion_items(&mut self, completion: Option<CompletionResponse>) {
        if let Some(res) = completion {
            let items = match res {
                CompletionResponse::Array(items) => items,
                CompletionResponse::List(list) => list.items,
            };
            self.completion_items.extend(items);
        }
    }

    pub fn collect_code_lens(&mut self, code_lens: Option<Vec<CodeLens>>) {
        if let Some(code_lens) = code_lens {
            self.code_lens.extend(code_lens);
        }
    }

    pub fn collect_workspace_symbols(
        &mut self,
        symbols: Option<Vec<WorkspaceSymbol>>,
        symbol_ranges: &mut HashSet<SymbolRange>,
    ) {
        if let Some(symbols) = symbols {
            self.workspace_symbols.extend(symbols.clone());
            symbol_ranges.extend(symbols.into_iter().filter_map(|sym| {
                if let OneOf::Left(it) = sym.location {
                    Some(SymbolRange::new(it.uri, it.range))
                } else {
                    None
                }
            }));
        }
    }

    pub fn collect_flat_symbol_ranges(
        symbol_infos: Option<Vec<SymbolInformation>>,
        symbol_ranges: &mut HashSet<SymbolRange>,
    ) {
        if let Some(symbols) = symbol_infos {
            symbol_ranges.extend(
                symbols
                    .into_iter()
                    .map(|sym| SymbolRange::new(sym.location.uri.clone(), sym.location.range)),
            );
        }
    }

    pub fn collect_type_hierarchy_items(&mut self, items: Option<Vec<TypeHierarchyItem>>) {
        if let Some(items) = items {
            self.type_hierarchy_items.extend(items);
        }
    }

    pub fn collect_call_hierarchy_items(&mut self, items: Option<Vec<CallHierarchyItem>>) {
        if let Some(items) = items {
            self.call_hierarchy_items.extend(items);
        }
    }

    pub fn collect_document_links(&mut self, links: Option<Vec<DocumentLink>>) {
        if let Some(links) = links {
            self.document_links.extend(links);
        }
    }
}

#[allow(clippy::unsafe_derive_deserialize)]
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
