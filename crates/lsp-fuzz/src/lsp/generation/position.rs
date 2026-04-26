use std::{marker::PhantomData, str::FromStr};

use derive_new::new as New;
use libafl::{
    HasMetadata,
    state::{HasCurrentTestcase, HasRand},
};
use libafl_bolts::rands::Rand;
use lsp_types::{TextDocumentIdentifier, TextDocumentPositionParams};

use super::{
    DynGenerator, GenerationError, HasGenerators, LspParamsGenerator, WeightedGeneratorList,
    boxed_generator,
    position_selectors::{
        HighlightSteer, NodeTypeBalancingSelection, PositionSelector, RandomPosition, ValidPosition,
    },
};
use crate::{
    lsp_input::{LspInput, server_response::metadata::LspResponseInfo},
    text_document::{
        GrammarBasedMutation, TextDocument,
        mutations::{core::TextDocumentSelector, text_document_selectors::RandomDoc},
    },
    utils::{ToLspPosition, ToLspRange, ToTreeSitterPoint, generate_random_uri_content},
};

#[derive(Debug)]
pub struct TextDocumentPositionParamsGenerator<D, PosSel> {
    position_selector: PosSel,
    _phantom: PhantomData<D>,
}

impl<D, PosSel> TextDocumentPositionParamsGenerator<D, PosSel> {
    pub const fn new(position_selector: PosSel) -> Self {
        Self {
            position_selector,
            _phantom: PhantomData,
        }
    }
}

impl<State, D, PosSel> LspParamsGenerator<State> for TextDocumentPositionParamsGenerator<D, PosSel>
where
    D: TextDocumentSelector<State>,
    PosSel: PositionSelector<State>,
{
    type Output = lsp_types::TextDocumentPositionParams;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let position = self
            .position_selector
            .select_position(state, doc)
            .ok_or(GenerationError::NothingGenerated)?;
        Ok(Self::Output {
            text_document: TextDocumentIdentifier { uri },
            position,
        })
    }
}

impl<State> HasGenerators<State> for TextDocumentPositionParams
where
    State: HasRand + HasMetadata + HasCurrentTestcase<LspInput>,
{
    type Generator = DynGenerator<State, Self>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        type SelectInRandomDoc<PosSel> = TextDocumentPositionParamsGenerator<RandomDoc, PosSel>;
        type FeedbackPosInDoc<F> = FeedbackPositionsGenerator<RandomDoc, F>;
        let term_start_pos = NodeTypeBalancingSelection::new();
        let node_type: Self::Generator = boxed_generator(SelectInRandomDoc::new(term_start_pos));
        let steer: Self::Generator = boxed_generator(SelectInRandomDoc::new(HighlightSteer::new()));
        let random_position = boxed_generator(SelectInRandomDoc::new(RandomPosition::new(1024)));
        let invalid_pos = boxed_generator(InvalidDocPositionGenerator::new());

        let mut generators = WeightedGeneratorList::with_capacity(16);
        if config.use_context() {
            let valid = boxed_generator(SelectInRandomDoc::new(ValidPosition::new()));
            generators.push_weighted(valid, 2);
            if config.use_grammar_ops() {
                generators.push_weighted(node_type.clone(), 3);
                generators.push_weighted(steer.clone(), 3);
            }
            if config.use_feedback_guidance() {
                generators.push(boxed_generator(FeedbackPosInDoc::new(diag_nodes)));
                generators
                    .push_weighted(boxed_generator(FeedbackPosInDoc::new(diag_nodes_parent)), 2);
                generators
                    .push_weighted(boxed_generator(FeedbackPosInDoc::new(collected_symbols)), 3);
            }
            if config.allow_invalid_positions() {
                generators.push(random_position);
            }
        } else {
            generators.push_weighted(invalid_pos, 4);
        }

        generators.finish()
    }
}

#[derive(Debug, New)]
pub struct InvalidDocPositionGenerator;

impl<State> LspParamsGenerator<State> for InvalidDocPositionGenerator
where
    State: HasRand,
{
    type Output = TextDocumentPositionParams;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let generate =
            |state: &mut State, _input: &LspInput| -> Option<TextDocumentPositionParams> {
                let rand = state.rand_mut();
                let uri_content = generate_random_uri_content(rand, 256);
                let random_uri = lsp_types::Uri::from(
                    fluent_uri::Uri::from_str(&format!("lsp-fuzz://{uri_content}")).ok()?,
                );

                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "65536 is within the range of u32"
                )]
                let position = lsp_types::Position {
                    line: rand.below_or_zero(65536) as u32,
                    character: rand.below_or_zero(65536) as u32,
                };

                Some(TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: random_uri },
                    position,
                })
            };
        generate(state, input).ok_or(GenerationError::NothingGenerated)
    }
}

#[derive(Debug, New)]
pub struct FeedbackPositionsGenerator<D, F> {
    position_selector: F,
    _doc_selector: PhantomData<D>,
}

impl<State, D, F> LspParamsGenerator<State> for FeedbackPositionsGenerator<D, F>
where
    D: TextDocumentSelector<State>,
    F: Fn(&LspResponseInfo, &lsp_types::Uri, &TextDocument) -> Vec<lsp_types::Position>,
    State: HasRand + HasCurrentTestcase<LspInput>,
{
    type Output = TextDocumentPositionParams;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let test_case = state
            .current_testcase()
            .map_err(|_| GenerationError::NothingGenerated)?;
        let response_info = test_case
            .metadata::<LspResponseInfo>()
            .map_err(|_| GenerationError::NothingGenerated)?;
        let points = (self.position_selector)(response_info, &uri, doc);
        drop(test_case);
        let position = state
            .rand_mut()
            .choose(points)
            .ok_or(GenerationError::NothingGenerated)?;

        Ok(Self::Output {
            text_document: TextDocumentIdentifier { uri },
            position,
        })
    }
}

pub(super) fn diag_nodes(
    data: &LspResponseInfo,
    uri: &lsp_types::Uri,
    doc: &TextDocument,
) -> Vec<lsp_types::Position> {
    data.diagnostics
        .iter()
        .filter(|diag| &diag.uri == uri)
        .flat_map(|diag| doc.node_starts_in_range(diag.range))
        .map(|it| it.to_lsp_position())
        .collect()
}

pub(super) fn diag_nodes_parent(
    data: &LspResponseInfo,
    uri: &lsp_types::Uri,
    doc: &TextDocument,
) -> Vec<lsp_types::Position> {
    data.diagnostics
        .iter()
        .filter(|diag| &diag.uri == uri)
        .filter_map(|it| {
            doc.parse_tree().root_node().descendant_for_point_range(
                it.range.start.to_ts_point(),
                it.range.end.to_ts_point(),
            )
        })
        .filter_map(|it| it.parent())
        .flat_map(|it| doc.node_starts_in_range(it.range().to_lsp_range()))
        .map(|it| it.to_lsp_position())
        .collect()
}

pub(super) fn collected_symbols(
    data: &LspResponseInfo,
    uri: &lsp_types::Uri,
    doc: &TextDocument,
) -> Vec<lsp_types::Position> {
    data.symbol_ranges
        .iter()
        .filter(|it| &it.uri == uri)
        .flat_map(|it| doc.node_starts_in_range(it.range))
        .map(|it| it.to_lsp_position())
        .collect()
}
