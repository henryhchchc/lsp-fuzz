pub(crate) mod capatibilities;
pub mod message;

pub use message::Message;

use crate::lsp_input::LspInput;
pub mod generation;
pub mod json_rpc;

pub trait LspParamsGen {
    fn generate_one<S>(state: &mut S, input: &LspInput) -> Self;
}

pub trait LspMessage {
    type Params;
    const METHOD: &'static str;
}

pub trait IntoMessage<M>
where
    M: LspMessage,
{
    fn into_message(params: M::Params) -> Message;
}
