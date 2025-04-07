use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok};
use lsp_fuzz::text_document::{
    GrammarContextLookup,
    grammars::{Grammar, GrammarContext},
};
use lsp_fuzz_grammars::Language;
use rayon::prelude::*;

pub fn load_grammar_context(
    lang: Language,
    derivation_fragment_file: &Path,
) -> Result<GrammarContext, anyhow::Error> {
    let file = File::open(derivation_fragment_file).context("Opening derivation fragment")?;
    let reader = zstd::Decoder::new(BufReader::new(file))?;
    let frags = serde_cbor::from_reader(reader).context("Deserializing derivation fragments")?;
    let grammar = Grammar::from_tree_sitter_grammar_json(lang, lang.grammar_json())?;
    let grammar_ctx = GrammarContext::new(grammar, frags);
    Ok(grammar_ctx)
}

pub fn load_grammar_lookup(
    lang_and_files: &HashMap<Language, PathBuf>,
) -> Result<GrammarContextLookup, anyhow::Error> {
    let contexts: Vec<_> = lang_and_files
        .iter()
        .par_bridge()
        .map(|(&lang, frag_path)| load_grammar_context(lang, frag_path))
        .try_fold(Vec::new, |mut acc, res| {
            res.map(|it| {
                acc.push(it);
                acc
            })
        })
        .try_reduce(Vec::default, |mut lhs, rhs| {
            lhs.extend(rhs);
            Ok(lhs)
        })?;
    let grammar_ctx = GrammarContextLookup::from_iter(contexts);
    Ok(grammar_ctx)
}
