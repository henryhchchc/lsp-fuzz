pub(crate) mod capabilities;
pub mod message;

use generation::{LspParamsGenerator, numeric::TabSizeGen};
pub use message::LspMessage;
use message::LspResponse;
use serde::{Deserialize, Serialize};

pub mod code_context;
pub mod compositions;
pub mod generation;
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

pub trait HasGenerators<State> {
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
    pub invalid_input: InvalidInputConfig,
    pub tab_size: TabSizeGen,
    pub awareness: AwarenessConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvalidInputConfig {
    pub ranges: bool,
    pub positions: bool,
    pub code_frequency: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwarenessConfig {
    pub grammar_ops: bool,
    pub context: bool,
    pub feedback_guidance: bool,
}

impl GeneratorsConfig {
    fn defaults() -> (InvalidInputConfig, TabSizeGen) {
        (
            InvalidInputConfig {
                ranges: true,
                positions: true,
                code_frequency: 0.1,
            },
            TabSizeGen {
                candidates: vec![0, 1, 2, 4, 8],
                rand_prob: 0.2,
            },
        )
    }

    #[must_use]
    pub fn full() -> Self {
        let (invalid_input, tab_size) = Self::defaults();
        Self {
            invalid_input,
            tab_size,
            awareness: AwarenessConfig {
                grammar_ops: true,
                context: true,
                feedback_guidance: true,
            },
        }
    }

    #[must_use]
    pub fn no_server_feedback() -> Self {
        let (invalid_input, tab_size) = Self::defaults();
        Self {
            invalid_input,
            tab_size,
            awareness: AwarenessConfig {
                grammar_ops: false,
                context: true,
                feedback_guidance: false,
            },
        }
    }

    #[must_use]
    pub fn no_context_awareness() -> Self {
        let (invalid_input, tab_size) = Self::defaults();
        Self {
            invalid_input,
            tab_size,
            awareness: AwarenessConfig {
                grammar_ops: true,
                context: false,
                feedback_guidance: false,
            },
        }
    }
}
