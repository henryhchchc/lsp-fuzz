pub(crate) mod capabilities;
pub mod message;

use generation::{LspParamsGenerator, numeric::TabSizeGen};
pub use message::ClientToServerMessage;
use serde::{Deserialize, Serialize};

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

    fn generators(config: &GeneratorsConfig) -> impl IntoIterator<Item = Self::Generator>;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorsConfig {
    pub invalid_ranges: bool,
    pub tab_size: TabSizeGen,
}

impl Default for GeneratorsConfig {
    fn default() -> Self {
        Self {
            invalid_ranges: true,
            tab_size: TabSizeGen {
                candidates: vec![0, 1, 2, 4, 8],
                rand_prob: 0.2,
            },
        }
    }
}
