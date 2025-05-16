use libafl::state::{HasCurrentTestcase, HasRand};
use lsp_types::{
    CallHierarchyItem, CodeAction, CodeLens, Command, CompletionItem, DocumentLink, InlayHint,
    TypeHierarchyItem, WorkspaceSymbol,
};
use trait_gen::trait_gen;

use super::meta::ParamFragmentGenerator;
use crate::{
    lsp::{GeneratorsConfig, HasPredefinedGenerators},
    lsp_input::LspInput,
};

#[trait_gen(T->
    CodeAction,
    Command,
    InlayHint,
    CompletionItem,
    CodeLens,
    WorkspaceSymbol,
    TypeHierarchyItem,
    CallHierarchyItem,
    DocumentLink,
)]
impl<State> HasPredefinedGenerators<State> for T
where
    State: HasCurrentTestcase<LspInput> + HasRand,
{
    type Generator = ParamFragmentGenerator<T>;

    fn generators(config: &GeneratorsConfig) -> impl IntoIterator<Item = Self::Generator> {
        [ParamFragmentGenerator::new(
            config.feedback_guidance && config.ctx_awareness,
        )]
    }
}
