use std::{borrow::Cow, collections::HashSet};

use derive_more::Debug;
use libafl::{
    HasMetadata,
    corpus::{Corpus, Testcase},
    events::EventFirer,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    state::{HasCorpus, HasExecutions},
};
use libafl_bolts::{
    Named,
    tuples::{Handle, Handled, MatchNameRef},
};
use lsp_types::notification::PublishDiagnostics;

fn collect_diagnostics(matching: &matching::RequestResponseMatching<'_>) -> HashSet<Diagnostic> {
    let mut diagnostics = HashSet::new();

    for pub_diag in matching.find_notifications::<PublishDiagnostics>() {
        let uri = LspInput::lift_uri(&pub_diag.uri);
        for diag_item in &pub_diag.diagnostics {
            let diag = Diagnostic {
                uri: uri.as_ref().clone(),
                range: diag_item.range,
            };
            diagnostics.insert(diag);
        }
    }

    diagnostics
}
use metadata::{Diagnostic, LspResponseInfo, ParamFragments, SymbolRange};
use tracing::warn;

use super::LspInput;
use crate::{
    execution::responses::LspOutputObserver,
    lsp::{LspMessage, message::LspResponse},
    utils::AflContext,
};

pub mod metadata;

pub mod matching;

#[derive(Debug)]
pub struct LspResponseFeedback {
    observer_handle: Handle<LspOutputObserver>,
}

impl LspResponseFeedback {
    #[must_use]
    pub fn new(observer: &LspOutputObserver) -> Self {
        Self {
            observer_handle: observer.handle(),
        }
    }
}

impl Named for LspResponseFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ResponseNoveltyFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for LspResponseFeedback where State: HasMetadata {}

impl<EM, Observers, State> Feedback<EM, LspInput, Observers, State> for LspResponseFeedback
where
    State: HasMetadata + HasExecutions + HasCorpus<LspInput>,
    Observers: MatchNameRef,
    EM: EventFirer<LspInput, State>,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &LspInput,
        _observers: &Observers,
        _exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        observers: &Observers,
        testcase: &mut Testcase<LspInput>,
    ) -> Result<(), libafl::Error> {
        state
            .corpus()
            .load_input_into(testcase)
            .afl_context("Loading input to the test case")?;
        let input = testcase
            .input()
            .as_ref()
            .expect("We loaded the input just now.");

        let response_observer = observers
            .get(&self.observer_handle)
            .afl_context("LspResponseObserver not attached")?;
        let received_messages = response_observer.captured_messages();
        let Ok(matching) = matching::RequestResponseMatching::match_messages(
            input.messages.iter(),
            received_messages.iter(),
        ) else {
            warn!("Failed to match messages");
            return Ok(());
        };

        let diagnostics = collect_diagnostics(&matching);

        let mut param_fragments = ParamFragments::default();
        let mut symbol_ranges = HashSet::new();

        for (req, res) in matching.responses {
            use LspResponse::*;
            match res {
                CodeActionRequest(cas) => {
                    param_fragments.collect_code_actions(cas);
                }
                InlayHintRequest(inlay_hints) => {
                    param_fragments.collect_inlay_hints(inlay_hints);
                }
                Completion(completion) => {
                    param_fragments.collect_completion_items(completion);
                }
                CodeLensRequest(code_lens) => {
                    param_fragments.collect_code_lens(code_lens);
                }
                WorkspaceSymbolRequest(Some(lsp_types::WorkspaceSymbolResponse::Nested(
                    symbols,
                ))) => {
                    param_fragments.collect_workspace_symbols(Some(symbols), &mut symbol_ranges);
                }
                WorkspaceSymbolRequest(Some(lsp_types::WorkspaceSymbolResponse::Flat(symbols)))
                | DocumentSymbolRequest(Some(lsp_types::DocumentSymbolResponse::Flat(symbols))) => {
                    ParamFragments::collect_flat_symbol_ranges(Some(symbols), &mut symbol_ranges);
                }
                DocumentSymbolRequest(Some(lsp_types::DocumentSymbolResponse::Nested(symbols))) => {
                    if let LspMessage::DocumentSymbolRequest(req) = req {
                        let mut queue = std::collections::VecDeque::from_iter(symbols);
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
                TypeHierarchyPrepare(items) => {
                    param_fragments.collect_type_hierarchy_items(items);
                }
                CallHierarchyPrepare(items) => {
                    param_fragments.collect_call_hierarchy_items(items);
                }
                DocumentLinkRequest(links) => {
                    param_fragments.collect_document_links(links);
                }
                _ => {}
            }
        }

        let response_info = LspResponseInfo {
            diagnostics,
            param_fragments,
            symbol_ranges,
        };
        testcase.add_metadata(response_info);
        Ok(())
    }
}
