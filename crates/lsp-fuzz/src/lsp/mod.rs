pub(crate) mod capabilities;
pub mod message;

use generation::LspParamsGenerator;
pub use message::ClientToServerMessage;

pub mod compositions;
pub mod generation;
pub mod json_rpc;

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

pub trait HasPredefinedGenerators<S> {
    type Generator: LspParamsGenerator<S, Output = Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>
    where
        S: 'static;
}

pub trait Compose {
    type Components;

    fn compose(components: Self::Components) -> Self;
}
