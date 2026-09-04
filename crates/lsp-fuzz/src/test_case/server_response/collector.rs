use std::collections::{HashSet, VecDeque};

use lsp_types::notification::PublishDiagnostics;

use super::{
    LspInput,
    matching::RequestResponseMatching,
    metadata::{Diagnostic, LspResponseInfo, ParamFragments, SymbolRange},
};
use crate::lsp::{LspMessage, message::LspResponse};

pub fn collect_response_info(matching: RequestResponseMatching<'_>) -> LspResponseInfo {
    let diagnostics = collect_diagnostics(&matching);
    let mut param_fragments = ParamFragments::default();
    let mut symbol_ranges = HashSet::new();

    for (req, res) in matching.responses {
        collect_response_fragments(req, res, &mut param_fragments, &mut symbol_ranges);
    }

    LspResponseInfo {
        diagnostics,
        param_fragments,
        symbol_ranges,
    }
}

fn collect_diagnostics(matching: &RequestResponseMatching<'_>) -> HashSet<Diagnostic> {
    let mut diagnostics = HashSet::new();

    for pub_diag in matching.find_notifications::<PublishDiagnostics>() {
        let uri = LspInput::lift_uri(&pub_diag.uri);
        for diag_item in &pub_diag.diagnostics {
            diagnostics.insert(Diagnostic {
                uri: uri.as_ref().clone(),
                range: diag_item.range,
            });
        }
    }

    diagnostics
}

fn collect_response_fragments(
    req: &LspMessage,
    res: LspResponse,
    param_fragments: &mut ParamFragments,
    symbol_ranges: &mut HashSet<SymbolRange>,
) {
    match res {
        LspResponse::CodeActionRequest(cas) => {
            param_fragments.collect_code_actions(cas);
        }
        LspResponse::InlayHintRequest(inlay_hints) => {
            param_fragments.collect_inlay_hints(inlay_hints);
        }
        LspResponse::Completion(completion) => {
            param_fragments.collect_completion_items(completion);
        }
        LspResponse::CodeLensRequest(code_lens) => {
            param_fragments.collect_code_lens(code_lens);
        }
        LspResponse::WorkspaceSymbolRequest(Some(lsp_types::WorkspaceSymbolResponse::Nested(
            symbols,
        ))) => {
            param_fragments.collect_workspace_symbols(Some(symbols), symbol_ranges);
        }
        LspResponse::WorkspaceSymbolRequest(Some(lsp_types::WorkspaceSymbolResponse::Flat(
            symbols,
        )))
        | LspResponse::DocumentSymbolRequest(Some(lsp_types::DocumentSymbolResponse::Flat(
            symbols,
        ))) => {
            ParamFragments::collect_flat_symbol_ranges(Some(symbols), symbol_ranges);
        }
        LspResponse::DocumentSymbolRequest(Some(lsp_types::DocumentSymbolResponse::Nested(
            symbols,
        ))) => {
            collect_nested_document_symbols(req, symbols, symbol_ranges);
        }
        LspResponse::TypeHierarchyPrepare(items) => {
            param_fragments.collect_type_hierarchy_items(items);
        }
        LspResponse::CallHierarchyPrepare(items) => {
            param_fragments.collect_call_hierarchy_items(items);
        }
        LspResponse::DocumentLinkRequest(links) => {
            param_fragments.collect_document_links(links);
        }
        _ => {}
    }
}

fn collect_nested_document_symbols(
    req: &LspMessage,
    symbols: Vec<lsp_types::DocumentSymbol>,
    symbol_ranges: &mut HashSet<SymbolRange>,
) {
    if let LspMessage::DocumentSymbolRequest(req) = req {
        let mut queue = VecDeque::from_iter(symbols);
        while let Some(symbol) = queue.pop_front() {
            let mut symbol = symbol.clone();
            if let Some(children) = symbol.children.take() {
                queue.extend(children);
            }
            symbol_ranges.insert(SymbolRange::new(
                req.text_document.uri.clone(),
                symbol.selection_range,
            ));
        }
    }
}
