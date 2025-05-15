use std::{borrow::Cow, collections::HashMap, hash::Hash, marker::PhantomData};

use derive_more::Debug;
use fastbloom::BloomFilter;
use libafl::{
    HasMetadata,
    events::{Event, EventFirer, EventWithStats},
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    monitors::stats::{AggregatorOps, UserStats, UserStatsValue},
    state::HasExecutions,
};
use libafl_bolts::{
    Named, SerdeAny,
    tuples::{Handle, Handled, MatchNameRef},
};
use lsp_types::PublishDiagnosticsParams;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::LspInput;
use crate::{
    execution::responses::LspOutputObserver,
    lsp::{
        LspMessage,
        json_rpc::{JsonRPCMessage, MessageId, ResponseError},
        message::{LspResponse, MessageDecodeError},
    },
    utils::AflContext,
};

#[derive(Debug)]
pub struct OutputNoveltyFeedback {
    observer_handle: Handle<LspOutputObserver>,
}

impl OutputNoveltyFeedback {
    pub fn new(observer: &LspOutputObserver) -> Self {
        Self {
            observer_handle: observer.handle(),
        }
    }
}

impl Named for OutputNoveltyFeedback {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ResponseNoveltyFeedback");
        &NAME
    }
}

impl<State> StateInitializer<State> for OutputNoveltyFeedback
where
    State: HasMetadata,
{
    fn init_state(&mut self, state: &mut State) -> Result<(), libafl::Error> {
        state.add_metadata(ObservedValues::default());
        Ok(())
    }
}

impl<EM, OT, State> Feedback<EM, LspInput, OT, State> for OutputNoveltyFeedback
where
    State: HasMetadata + HasExecutions,
    OT: MatchNameRef,
    EM: EventFirer<LspInput, State>,
{
    fn is_interesting(
        &mut self,
        state: &mut State,
        manager: &mut EM,
        input: &LspInput,
        observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, libafl::Error> {
        let observed_values = state
            .metadata_mut::<ObservedValues>()
            .afl_context("Metadata not found")?;

        let observer = observers
            .get(&self.observer_handle)
            .afl_context("ResponseObserver not attached")?;
        let messages = observer.captured_messages();

        let mut is_interesting = false;

        let Ok(RequestResponseMatching {
            // responses,
            notifications,
            ..
        }) = RequestResponseMatching::match_messages(input.messages.as_ref(), messages)
        else {
            warn!("Got unexpected messages from target");
            return Ok(false);
        };

        for n in notifications {
            if let LspMessage::PublishDiagnostics(PublishDiagnosticsParams {
                diagnostics, ..
            }) = n
            {
                for lsp_types::Diagnostic {
                    severity,
                    code,
                    source,
                    ..
                } in diagnostics
                {
                    let val = ("textDocument/publishDiagnostics", severity, code, source);
                    if observed_values.merge(&val) {
                        observed_values.seen_diagnostics += 1;
                        is_interesting = true;
                    }
                }
            }
        }
        let stat_value = UserStatsValue::Number(observed_values.seen_diagnostics as u64);
        let exec = *(state.executions());
        manager.fire(
            state,
            EventWithStats::with_current_time(
                Event::UpdateUserStats {
                    name: Cow::Borrowed("seen_diagnostics"),
                    value: UserStats::new(stat_value, AggregatorOps::Max),
                    phantom: PhantomData,
                },
                exec,
            ),
        )?;

        Ok(is_interesting)
    }

    fn append_metadata(
        &mut self,
        _state: &mut State,
        _manager: &mut EM,
        _observers: &OT,
        _testcase: &mut libafl::corpus::Testcase<LspInput>,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, SerdeAny)]
pub struct ObservedValues {
    inner: BloomFilter,
    seen_diagnostics: usize,
}

impl Default for ObservedValues {
    fn default() -> Self {
        Self {
            inner: BloomFilter::with_false_pos(0.001).expected_items(1_000_000),
            seen_diagnostics: 0,
        }
    }
}

impl ObservedValues {
    pub fn merge(&mut self, value: &(impl Hash + ?Sized)) -> bool {
        !self.inner.insert(&value)
    }
}

#[derive(Debug)]
pub struct RequestResponseMatching<'a> {
    pub responses: HashMap<&'a LspMessage, LspResponse>,
    pub errors: HashMap<&'a LspMessage, ResponseError>,
    pub notifications: Vec<LspMessage>,
    pub requests_from_server: Vec<LspMessage>,
}

impl<'a> RequestResponseMatching<'a> {
    fn match_messages(
        sent_messages: &'a [LspMessage],
        received_messages: &[JsonRPCMessage],
    ) -> Result<Self, MessageDecodeError> {
        let mut responses = HashMap::new();
        let mut notifications = Vec::new();
        let mut requests_from_server = Vec::new();
        let mut errors = HashMap::new();

        let requests: HashMap<_, _> = sent_messages
            .iter()
            .filter(|it| it.is_request())
            .enumerate()
            .map(|(id, msg)| (MessageId::Number(id + 2), msg))
            .collect();

        for recv in received_messages {
            match recv {
                JsonRPCMessage::Request { method, params, .. } => {
                    let request = LspMessage::try_from_json(method, params.clone())?;
                    requests_from_server.push(request);
                }
                JsonRPCMessage::Notification { method, params, .. } => {
                    let notification = LspMessage::try_from_json(method, params.clone())?;
                    notifications.push(notification);
                }
                JsonRPCMessage::Response {
                    id: Some(id),
                    result,
                    error,
                    ..
                } => {
                    if let Some(corresponding_request) = requests.get(id).copied() {
                        if let Some(result) = result {
                            let response = LspResponse::try_from_json(
                                corresponding_request.method(),
                                result.clone(),
                            )?;
                            responses.insert(corresponding_request, response);
                        } else if let Some(error) = error {
                            errors.insert(corresponding_request, error.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            responses,
            notifications,
            errors,
            requests_from_server,
        })
    }
}
