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
use lsp_types::{CompletionResponse, WorkspaceSymbolResponse, notification::PublishDiagnostics};
use metadata::{Diagnostic, LspResponseInfo, ParamFragments};
use tracing::warn;

use super::LspInput;
use crate::{
    execution::responses::LspOutputObserver, lsp::message::LspResponse, utils::AflContext,
};

pub mod metadata;

pub mod matching;

#[derive(Debug)]
pub struct LspResponseFeedback {
    observer_handle: Handle<LspOutputObserver>,
}

impl LspResponseFeedback {
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

impl<EM, OT, State> Feedback<EM, LspInput, OT, State> for LspResponseFeedback
where
    State: HasMetadata + HasExecutions + HasCorpus<LspInput>,
    OT: MatchNameRef,
    EM: EventFirer<LspInput, State>,
{
    fn is_interesting(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _input: &LspInput,
        _observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut State,
        _manager: &mut EM,
        observers: &OT,
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

        let mut diagnostics = HashSet::new();

        for pub_diag in matching.find_notifications::<PublishDiagnostics>() {
            let uri = LspInput::lift_uri(&pub_diag.uri);
            for diag_item in pub_diag.diagnostics.iter() {
                let diag = Diagnostic {
                    uri: uri.as_ref().clone(),
                    range: diag_item.range,
                };
                diagnostics.insert(diag);
            }
        }

        let mut param_fragments = ParamFragments::default();

        for res in matching.responses.values() {
            use LspResponse::*;
            match res {
                CodeActionRequest(Some(cas)) => cas.iter().cloned().for_each(|ca| match ca {
                    lsp_types::CodeActionOrCommand::Command(command) => {
                        param_fragments.commands.insert(command);
                    }
                    lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
                        param_fragments.code_actions.insert(code_action);
                    }
                }),
                InlayHintRequest(Some(inlay_hints)) => {
                    param_fragments
                        .inlay_hints
                        .extend(inlay_hints.iter().cloned());
                }
                Completion(Some(res)) => {
                    let items = match res {
                        CompletionResponse::Array(items) => items,
                        CompletionResponse::List(list) => &list.items,
                    };
                    param_fragments
                        .completion_items
                        .extend(items.iter().cloned());
                }
                CodeLensRequest(Some(code_lens)) => {
                    param_fragments.code_lens.extend(code_lens.iter().cloned());
                }
                WorkspaceSymbolRequest(Some(WorkspaceSymbolResponse::Nested(symbols))) => {
                    param_fragments
                        .workspace_symbols
                        .extend(symbols.iter().cloned());
                }
                TypeHierarchyPrepare(Some(items)) => {
                    param_fragments
                        .type_hierarchy_items
                        .extend(items.iter().cloned());
                }
                CallHierarchyPrepare(Some(items)) => {
                    param_fragments
                        .call_hierarchy_items
                        .extend(items.iter().cloned());
                }
                DocumentLinkRequest(Some(links)) => {
                    param_fragments.document_links.extend(links.iter().cloned());
                }
                _ => {}
            }
        }

        let response_info = LspResponseInfo {
            diagnostics,
            param_fragments,
        };
        testcase.add_metadata(response_info);
        Ok(())
    }
}
