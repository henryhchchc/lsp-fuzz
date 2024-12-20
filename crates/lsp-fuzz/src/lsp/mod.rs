pub(crate) mod capabilities;
pub mod message;

pub use message::Message;

pub mod generation;
pub mod json_rpc;

pub mod workspace_localization;

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
    fn localize(&mut self, workspace_dir: &str);
}
