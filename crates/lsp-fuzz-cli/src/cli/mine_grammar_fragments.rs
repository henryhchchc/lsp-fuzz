use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use anyhow::Context;
use itertools::Itertools;
use lsp_fuzz::text_document::{
    grammars::{fragment_extraction::extract_derivation_fragments, DerivationFragments},
    Language,
};
use rayon::prelude::*;

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

        let extracted_fragements: Vec<_> = source_files
            .into_par_iter()
            .map(|source_file_path| -> anyhow::Result<_> {
                let file_content = std::fs::read(&source_file_path)
                    .with_context(|| format!("Reading: {}", source_file_path.display()))?;
                let mut parser = self.language.tree_sitter_parser();
                let fragments = extract_derivation_fragments(&file_content, &mut parser)
                    .with_context(|| {
                        format!(
                            "Extracting derivation fragments from {}",
                            source_file_path.display()
                        )
                    })?;
                Ok((file_content, fragments))
            })
            .collect::<Result<_, _>>()?;
        let mut code = Vec::new();
        let mut fragments = HashMap::new();

        for (file_content, file_fragments) in extracted_fragements {
            let offset = code.len();
            code.extend(file_content);
            for (node_kind, ranges) in file_fragments {
                let ranges = ranges
                    .into_iter()
                    .map(|range| (range.start + offset)..(range.end + offset));
                fragments
                    .entry(node_kind)
                    .or_insert_with(HashSet::new)
                    .extend(ranges);
            }
        }

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
                it.path()
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
    let mut zstd_encoder =
        zstd::Encoder::new(output_writer, 19).context("Creating zstd encoder")?;
    zstd_encoder
        .multithread(zstd_threads as u32)
        .context("Setting zstd encoder threads")?;
    let zstd_encoder = zstd_encoder.auto_finish();
    serde_cbor::to_writer(zstd_encoder, &result).context("Serializing derivation fragments")?;
    Ok(())
}
