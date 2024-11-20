pub(crate) mod capatibilities;
pub mod message;

pub use message::Message;

use crate::lsp_input::LspInput;
pub mod generation;
pub mod json_rpc;

pub mod workspace_localization;
pub trait LspParamsGen {
    fn generate_one<S>(state: &mut S, input: &LspInput) -> Self;
}

pub trait LspMessage {
    type Params;
    const METHOD: &'static str;
}

pub trait MessageParam<M>
where
    M: LspMessage,
{
    fn into_message(self) -> Message;
}

pub trait LocalizeToWorkspace {
    fn localize(self, workspace_dir: &str) -> Self;
}

