use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::BufWriter,
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::Context;
use itertools::Itertools;
use lsp_fuzz::text_document::grammars::{
    DerivationFragments,
    fragment_extraction::{self, extract_derivation_fragments},
};
use lsp_fuzz_grammars::Language;
use rayon::prelude::*;
use tracing::{info, warn};

use super::GlobalOptions;

/// Extracts derivation fragments from a set of source files
#[derive(Debug, clap::Parser)]
pub(super) struct MineGrammarFragments {
    /// The directory to search for source files
    #[clap(long, short)]
    search_directory: PathBuf,

    /// The language to use for parsing the source files
    #[clap(long, short)]
    language: Language,

    /// The output file to write the extracted fragments to
    #[clap(long, short, default_value = "fragments.cbor.zst")]
    output: PathBuf,
}

impl MineGrammarFragments {
    pub(super) fn run(self, global_options: GlobalOptions) -> anyhow::Result<()> {
        let source_files = self.find_source_files()?;

        info!("Found {} source files", source_files.len());
        let extracted_fragments: Vec<_> = source_files
            .into_par_iter()
            .inspect(|source_file_path| info!("Parsing: {}", source_file_path.display()))
            .map(|source_file| extract_fragmemts(&source_file, &self.language))
            .filter_map(|it| it.transpose())
            .collect::<Result<_, _>>()?;
        let mut code = Vec::new();
        let mut fragments = HashMap::new();

        info!("Merging fragments");
        for (file_content, file_fragments) in extracted_fragments {
            let offset = code.len();
            code.extend(file_content);
            for (node_kind, ranges) in file_fragments {
                let ranges = ranges
                    .into_iter()
                    .map(|range| (range.start + offset)..(range.end + offset));
                fragments
                    .entry(node_kind)
                    .or_insert_with(Vec::new)
                    .extend(ranges);
            }
        }

        info!("Deduplicating fragments");
        fragments.values_mut().par_bridge().for_each(|ranges| {
            ranges.sort_by_key(|it| &code[it.clone()]);
            ranges.dedup_by_key(|it| &code[it.clone()]);
        });

        info!("Serializing fragments");
        let result = DerivationFragments::new(code, fragments);
        write_output(&self.output, result, global_options.parallel_workers())
            .context("Writing output")?;

        Ok(())
    }

    fn find_source_files(&self) -> Result<Vec<PathBuf>, anyhow::Error> {
        let extensions = self.language.file_extensions();
        let source_files: Vec<_> = walkdir::WalkDir::new(&self.search_directory)
            .into_iter()
            .filter_ok(|it| {
                it.metadata().is_ok_and(|it| it.is_file())
                    && it
                        .path()
                        .extension()
                        .map(|it| it.to_string_lossy())
                        .is_some_and(|ext| extensions.contains(ext.as_ref()))
            })
            .map_ok(|it| it.into_path())
            .try_collect()
            .context("Searching for source file")?;
        Ok(source_files)
    }
}

fn write_output(
    output_path: &Path,
    result: DerivationFragments,
    zstd_threads: usize,
) -> Result<(), anyhow::Error> {
    let output_file = File::create(output_path).context("Creating output file")?;
    let output_writer = BufWriter::new(output_file);
    let zstd_encoder = {
        let mut enc = zstd::Encoder::new(output_writer, 19).context("Creating zstd encoder")?;
        enc.multithread(zstd_threads as u32)
            .context("Setting zstd encoder threads")?;
        enc.auto_finish()
    };
    serde_cbor::to_writer(zstd_encoder, &result).context("Serializing derivation fragments")?;
    Ok(())
}

type ExtractedFragments<'a> = (Vec<u8>, HashMap<Cow<'a, str>, Vec<Range<usize>>>);

fn extract_fragmemts<'a>(
    source_file_path: &Path,
    self_language: &Language,
) -> anyhow::Result<Option<ExtractedFragments<'a>>> {
    let file_content = std::fs::read(source_file_path)
        .with_context(|| format!("Reading: {}", source_file_path.display()))?;
    let mut parser = self_language.tree_sitter_parser();
    match extract_derivation_fragments(&file_content, &mut parser) {
        Ok(fragemnts) => Ok(Some((file_content, fragemnts))),
        Err(fragment_extraction::Error::DotGraphParsing(msg)) => {
            warn!(
                file = % source_file_path.display(),
                "Failed to parse dot graph: {}",
                msg,
            );
            Ok(None)
        }
        Err(e) => Err(e).with_context(|| {
            format!(
                "Extracting derivation fragments from {}",
                source_file_path.display()
            )
        }),
    }
}
