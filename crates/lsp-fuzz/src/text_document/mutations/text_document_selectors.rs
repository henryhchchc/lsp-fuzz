use std::option::Option;

use libafl::state::HasRand;
use libafl_bolts::rands::Rand;
use lsp_types::Uri;

use super::TextDocumentSelector;
use crate::{lsp_input::LspInput, text_document::TextDocument};

#[derive(Debug)]
pub struct RandomDoc;

impl<State> TextDocumentSelector<State> for RandomDoc
where
    State: HasRand,
{
    fn select_document<'i>(
        state: &mut State,
        input: &'i LspInput,
    ) -> Option<(Uri, &'i TextDocument)> {
        let iter = input.workspace.iter_files().filter_map(|(path, doc)| {
            doc.as_source_file().map(|doc| {
                (
                    format!("lsp-fuzz://{}", path.display()).parse().unwrap(),
                    doc,
                )
            })
        });
        state.rand_mut().choose(iter)
    }

    fn select_document_mut<'i>(
        state: &mut State,
        input: &'i mut LspInput,
    ) -> Option<(Uri, &'i mut TextDocument)> {
        let iter = input.workspace.iter_files_mut().filter_map(|(path, doc)| {
            doc.as_source_file_mut().map(|doc| {
                (
                    format!("lsp-fuzz://{}", path.display()).parse().unwrap(),
                    doc,
                )
            })
        });
        state.rand_mut().choose(iter)
    }
}
