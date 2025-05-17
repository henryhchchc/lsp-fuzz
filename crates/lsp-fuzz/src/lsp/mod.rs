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
pub mod metamodel;
pub mod ucc;

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
    pub invalid_positions: bool,
    pub invalid_code_frequency: f64,
    pub ctx_awareness: bool,
    pub feedback_guidance: bool,
}

impl GeneratorsConfig {
    pub fn full() -> Self {
        Self {
            invalid_ranges: true,
            invalid_positions: true,
            invalid_code_frequency: 0.1,
            ctx_awareness: true,
            feedback_guidance: true,
            tab_size: TabSizeGen {
                candidates: vec![0, 1, 2, 4, 8],
                rand_prob: 0.2,
            },
        }
    }

    pub fn no_server_feedback() -> Self {
        Self {
            invalid_ranges: true,
            invalid_positions: true,
            invalid_code_frequency: 0.1,
            ctx_awareness: true,
            feedback_guidance: false,
            tab_size: TabSizeGen {
                candidates: vec![0, 1, 2, 4, 8],
                rand_prob: 0.2,
            },
        }
    }

    pub fn no_context_awareness() -> Self {
        Self {
            invalid_ranges: true,
            invalid_positions: true,
            ctx_awareness: false,
            invalid_code_frequency: 0.1,
            feedback_guidance: false,
            tab_size: TabSizeGen {
                candidates: vec![0, 1, 2, 4, 8],
                rand_prob: 0.2,
            },
        }
    }
}
