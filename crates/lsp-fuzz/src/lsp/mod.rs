pub(crate) mod capabilities;
pub mod message;

use generation::LspParamsGenerator;
pub use message::ClientToServerMessage;

pub mod compositions;
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
    fn into_message(self) -> ClientToServerMessage;
}

pub trait LocalizeToWorkspace {
    fn localize(&mut self, workspace_dir: &str);
}

pub trait HasPredefinedGenerators<S> {
    type Generator: LspParamsGenerator<S, Output = Self>;

    fn generators() -> Vec<Self::Generator>
    where
        S: 'static;
}

pub trait Compose {
    type Components;

    fn compose(components: Self::Components) -> Self;
}
