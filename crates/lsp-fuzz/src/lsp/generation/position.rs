use std::{marker::PhantomData, rc::Rc, str::FromStr};

use derive_new::new as New;
use libafl::{
    HasMetadata,
    state::{HasCurrentTestcase, HasRand},
};
use libafl_bolts::rands::Rand;
use lsp_types::{TextDocumentIdentifier, TextDocumentPositionParams};

use super::{GenerationError, HasPredefinedGenerators, LspParamsGenerator};
use crate::{
    lsp::generation::meta::FallbackGenerator,
    lsp_input::{
        LspInput,
        messages::{
            HighlightSteer, PositionSelector, RandomPosition, TerminalStartPosition, ValidPosition,
        },
        server_response::metadata::LspResponseInfo,
    },
    text_document::mutations::{core::TextDocumentSelector, text_document_selectors::RandomDoc},
    utils::generate_random_uri_content,
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

impl<State> HasPredefinedGenerators<State> for TextDocumentPositionParams
where
    State: HasRand + HasMetadata + HasCurrentTestcase<LspInput>,
{
    type Generator = Rc<dyn LspParamsGenerator<State, Output = Self>>;

    fn generators(
        config: &crate::lsp::GeneratorsConfig,
    ) -> impl IntoIterator<Item = Self::Generator> {
        type SelectInRandomDoc<PosSel> = TextDocumentPositionParamsGenerator<RandomDoc, PosSel>;
        let term_start_pos = TerminalStartPosition::new();
        let term_start: Self::Generator = Rc::new(SelectInRandomDoc::new(term_start_pos));
        let steer: Self::Generator = Rc::new(SelectInRandomDoc::new(HighlightSteer::new()));
        let random_position = Rc::new(SelectInRandomDoc::new(RandomPosition::new(1024)));
        let invalid_pos = Rc::new(InvalidDocPositionGenerator::new());

        let diag1 = FallbackGenerator::new(
            DiagnosticPositionGenerator::<RandomDoc>::new(),
            SelectInRandomDoc::new(term_start_pos),
        );
        let diag2 = FallbackGenerator::new(
            DiagnosticPositionGenerator::<RandomDoc>::new(),
            SelectInRandomDoc::new(ValidPosition::new()),
        );
        let diag3 = FallbackGenerator::new(
            DiagnosticPositionGenerator::<RandomDoc>::new(),
            SelectInRandomDoc::new(HighlightSteer::new()),
        );

        let mut generators = Vec::new();
        if config.ctx_awareness {
            generators.extend([
                Rc::new(SelectInRandomDoc::new(ValidPosition::new())),
                Rc::new(SelectInRandomDoc::new(ValidPosition::new())),
                term_start.clone(),
                term_start.clone(),
                term_start.clone(),
                steer.clone(),
                steer.clone(),
                steer.clone(),
            ]);
            if config.feedback_guidance {
                generators.extend([
                    Rc::new(diag1) as _,
                    Rc::new(diag2) as _,
                    Rc::new(diag2) as _,
                    Rc::new(diag3) as _,
                    Rc::new(diag3) as _,
                ]);
            }
            if config.invalid_positions {
                generators.push(random_position);
            }
        } else {
            generators.push(invalid_pos.clone());
            generators.push(invalid_pos.clone());
            generators.push(invalid_pos.clone());
            generators.push(invalid_pos.clone());
        }

        generators
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
pub struct DiagnosticPositionGenerator<D> {
    _doc_selector: PhantomData<D>,
}

impl<State, D> LspParamsGenerator<State> for DiagnosticPositionGenerator<D>
where
    D: TextDocumentSelector<State>,
    State: HasRand + HasCurrentTestcase<LspInput>,
{
    type Output = TextDocumentPositionParams;

    fn generate(
        &self,
        state: &mut State,
        input: &LspInput,
    ) -> Result<Self::Output, GenerationError> {
        let (uri, _doc) =
            D::select_document(state, input).ok_or(GenerationError::NothingGenerated)?;
        let test_case = state
            .current_testcase()
            .map_err(|_| GenerationError::NothingGenerated)?;
        let response_info = test_case
            .metadata::<LspResponseInfo>()
            .map_err(|_| GenerationError::NothingGenerated)?;
        let diagnostics: Vec<_> = response_info
            .diagnostics
            .iter()
            .filter(|diag| diag.uri == uri)
            .map(|diag| diag.range.start)
            .collect();
        drop(test_case);
        let position = state
            .rand_mut()
            .choose(diagnostics)
            .ok_or(GenerationError::NothingGenerated)?;

        Ok(Self::Output {
            text_document: TextDocumentIdentifier { uri },
            position,
        })
    }
}
