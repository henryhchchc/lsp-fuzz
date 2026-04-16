use std::{marker::PhantomData, rc::Rc, result::Result, str::FromStr};

use derive_new::new as New;
use libafl::state::{HasCurrentTestcase, HasRand};
use libafl_bolts::rands::Rand;
use lsp_types::{Range, TextDocumentIdentifier, Uri};

use super::{FallbackGenerator, GenerationError, GeneratorBag, LspParamsGenerator};
use crate::{
    lsp::HasPredefinedGenerators,
    lsp_input::LspInput,
    text_document::{
        TextDocument,
        mutations::{core::TextDocumentSelector, text_document_selectors::RandomDoc},
    },
    utils::generate_random_uri_content,
};

mod range_selectors;

#[derive(Debug)]
pub struct Selection(pub TextDocumentIdentifier, pub Range);

#[derive(Debug, New)]
pub struct RangeInDocGenerator<State, D = RandomDoc> {
    range_selector: fn(&mut State, &Uri, &TextDocument) -> Range,
    _phantom: PhantomData<D>,
}

impl<State, D> Clone for RangeInDocGenerator<State, D> {
    fn clone(&self) -> Self {
        Self::new(self.range_selector)
    }
}

#[derive(Debug, New)]
pub struct InvalidSelectionGenerator;

impl<State, D> LspParamsGenerator<State> for RangeInDocGenerator<State, D>
where
    D: TextDocumentSelector<State>,
{
    type Output = Selection;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let range = (self.range_selector)(state, &uri, doc);
        let doc = TextDocumentIdentifier { uri };
        Ok(Selection(doc, range))
    }
}

impl<State> LspParamsGenerator<State> for InvalidSelectionGenerator
where
    State: HasRand,
{
    type Output = Selection;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let generate = |state: &mut State, _input: &LspInput| -> Option<Selection> {
            let rand = state.rand_mut();
            let uri_content = generate_random_uri_content(rand, 256);
            let random_uri = lsp_types::Uri::from(
                fluent_uri::Uri::from_str(&format!("lsp-fuzz://{uri_content}")).ok()?,
            );
            let mut random_pos = || -> lsp_types::Position {
                lsp_types::Position {
                    line: rand.below_or_zero(1024) as u32,
                    character: rand.below_or_zero(1024) as u32,
                }
            };
            let start = random_pos();
            let end = random_pos();

            Some(Selection(
                TextDocumentIdentifier { uri: random_uri },
                Range { start, end },
            ))
        };
        generate(state, input).ok_or(GenerationError::NothingGenerated)
    }
}

impl<State> HasPredefinedGenerators<State> for Selection
where
    State: HasRand + HasCurrentTestcase<LspInput> + 'static,
{
    type Generator = Rc<dyn LspParamsGenerator<State, Output = Selection>>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator>
    where
        State: HasRand,
    {
        let mut generators: GeneratorBag<Self::Generator> = GeneratorBag::with_capacity(16);

        type RINDGen<State> = RangeInDocGenerator<State, RandomDoc>;
        if config.ctx_awareness {
            generators.push(Rc::new(RINDGen::new(range_selectors::whole_range)) as _);
            generators.push_weighted(
                Rc::new(RINDGen::new(range_selectors::random_valid_range)) as _,
                2,
            );
            if config.grammar_ops_awareness {
                generators.push_weighted(
                    Rc::new(RINDGen::new(range_selectors::subtree_node_type)) as _,
                    5,
                );
            }
            if config.invalid_ranges {
                generators.extend(
                    [
                        RINDGen::new(range_selectors::after_range),
                        RINDGen::new(range_selectors::inverted_range),
                        RINDGen::new(range_selectors::random_invalid_range::<256, _>),
                    ]
                    .into_iter()
                    .map(Rc::new)
                    .map(|it| it as _),
                );
            }
            if config.feedback_guidance {
                let subtree = RINDGen::new(range_selectors::subtree_node_type);
                let fallback = |range_gen| FallbackGenerator::new(range_gen, subtree.clone());

                generators
                    .push(Rc::new(fallback(RINDGen::new(range_selectors::diagnosed_range))) as _);
                generators.push_weighted(
                    Rc::new(fallback(RINDGen::new(range_selectors::diagnosed_parent))) as _,
                    2,
                );
                generators.push_weighted(
                    Rc::new(fallback(RINDGen::new(range_selectors::symbols_range))) as _,
                    4,
                );
            }
        } else {
            generators.push_weighted(Rc::new(InvalidSelectionGenerator::new()) as _, 5);
        };

        generators.finish()
    }
}
