use lsp_types::{
    CodeActionKind, CodeActionTriggerKind, CompletionTriggerKind, SetTraceParams,
    SignatureHelpTriggerKind, TraceValue,
};

use super::{GenerationError, LspParamsGenerator};
use crate::lsp_input::LspInput;

#[derive(Debug, Clone)]
pub struct ConstGenerator<T> {
    value: T,
}

impl<T> ConstGenerator<T> {
    pub const fn new(value: T) -> Self {
        Self { value }
    }
}

impl<State, T> LspParamsGenerator<State> for ConstGenerator<T>
where
    T: Clone,
{
    type Output = T;

    fn generate(
        &self,
        _state: &mut State,
        _input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        Ok(self.value.clone())
    }
}

macro_rules! const_generators {
    (one $val: expr) => {1};
    (for $type: ty => [
        $($val: expr),*
    ]) => {
        impl<State> crate::lsp::HasPredefinedGenerators<State> for $type {
            type Generator = &'static ConstGenerator<Self>;

            fn generators() -> impl IntoIterator<Item = Self::Generator>
            where
                State: 'static,
            {
                const COUNT: usize = { 0 $( + const_generators!(one $val))* };
                static GENERATORS: [ConstGenerator<$type>; COUNT] = [
                    $(ConstGenerator::new($val)),*
                ];
                &GENERATORS
            }
        }
    };
}

const_generators!(for CompletionTriggerKind => [
    CompletionTriggerKind::INVOKED,
    CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS,
    CompletionTriggerKind::TRIGGER_CHARACTER
]);

const_generators!(for CodeActionTriggerKind => [
    CodeActionTriggerKind::INVOKED,
    CodeActionTriggerKind::AUTOMATIC
]);

const_generators!(for CodeActionKind => [
    CodeActionKind::EMPTY,
    CodeActionKind::QUICKFIX,
    CodeActionKind::REFACTOR,
    CodeActionKind::REFACTOR_EXTRACT,
    CodeActionKind::REFACTOR_INLINE,
    CodeActionKind::REFACTOR_REWRITE,
    CodeActionKind::SOURCE,
    CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
    CodeActionKind::SOURCE_FIX_ALL
]);

const_generators!(for SignatureHelpTriggerKind => [
    SignatureHelpTriggerKind::INVOKED,
    SignatureHelpTriggerKind::TRIGGER_CHARACTER,
    SignatureHelpTriggerKind::CONTENT_CHANGE
]);

const_generators!(for SetTraceParams => [
    SetTraceParams { value: TraceValue::Messages },
    SetTraceParams { value: TraceValue::Off },
    SetTraceParams { value: TraceValue::Verbose }
]);

const_generators!(for bool => [true, false]);
