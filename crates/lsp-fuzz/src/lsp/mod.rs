pub(crate) mod capabilities;
pub mod message;

pub use message::LspMessage;
use message::LspResponse;

pub mod code_context;
pub mod json_rpc;

pub trait LspMessageMeta {
    type Params;
    const METHOD: &'static str;
}

pub trait LspRequestMeta: LspMessageMeta {
    type Response;
}

pub trait MessageParam<M>
where
    M: LspMessageMeta,
{
    fn into_message(self) -> LspMessage;

    fn from_message_ref(message: &LspMessage) -> Option<&Self>;
}

pub trait MessageResponse<M>
where
    M: LspRequestMeta,
{
    fn from_response_ref(response: &LspResponse) -> Option<&Self>;
}
