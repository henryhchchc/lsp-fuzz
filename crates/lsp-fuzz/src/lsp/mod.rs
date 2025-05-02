pub(crate) mod capabilities;
pub mod message;

use generation::LspParamsGenerator;
pub use message::ClientToServerMessage;

pub mod code_context;
pub mod compositions;
pub mod generation;
pub mod json_rpc;
pub mod metamodel;

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

pub trait HasPredefinedGenerators<State> {
    type Generator: LspParamsGenerator<State, Output = Self>;

    fn generators() -> impl IntoIterator<Item = Self::Generator>;
}

pub trait Compose {
    type Components;

    fn compose(components: Self::Components) -> Self;
}

impl<Head, Tail> Compose for (Head, Tail) {
    type Components = (Head, Tail);

    #[inline]
    fn compose(components: Self::Components) -> Self {
        components
    }
}
