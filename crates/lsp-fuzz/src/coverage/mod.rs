use std::{
    fs::{self},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Context;
use derive_new::new as New;
use libafl::corpus::CorpusId;
use tempfile::TempDir;
use tracing::info;

const RAW_COVERAGE_DATA_FILE_EXT: &str = "profraw";

#[derive(Debug, New)]
pub struct CoverageDataGenerator {
    executable: PathBuf,
    args: Vec<String>,
}

impl CoverageDataGenerator {
    /// /// Measures the coverage for a given input.
    ///
    /// This function takes an input and a converter to convert it to bytes, and measures the coverage
    /// by executing the target program with the input. The coverage data is stored at the specified path.
    ///
    /// # Arguments
    ///
    /// * `input` - The input to measure coverage for
    /// * `converter` - Converter to convert the input to bytes
    /// * `coverage_data_path` - Path where the coverage data will be stored
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the coverage measurement is successful, or an error if something goes wrong.
    pub fn generate_llvm_profdata<I>(&self, inputs: I, merged_file: &Path) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = (CorpusId, Vec<u8>)>,
    {
        let temp_dir = TempDir::new().context("Creating tempdir")?;

        for (CorpusId(id), input) in inputs {
            let llvm_profile_raw = format!(
                "{}/coverage.{id}.{}",
                temp_dir.path().display(),
                RAW_COVERAGE_DATA_FILE_EXT
            );
            info!("Generating {llvm_profile_raw}");
            self.run_target_with_coverage(&input, &llvm_profile_raw)?;
            info!("Merging {llvm_profile_raw} into {}", merged_file.display());
            Command::new("llvm-profdata")
                .args(["merge", "-sparse"])
                .arg("-o")
                .arg(merged_file)
                .arg(merged_file)
                .arg(&llvm_profile_raw)
                .status()
                .context("Running llvm-profdata")?;
            fs::remove_file(&llvm_profile_raw).context("Removing temp raw data")?;
        }

        Ok(())
    }

    fn run_target_with_coverage(
        &self,
        input_bytes: &[u8],
        llvm_profile_raw: &str,
    ) -> Result<(), anyhow::Error> {
        let working_dir = TempDir::new().context("Creating temp working dir")?;
        let mut process = Command::new(&self.executable)
            .args(&self.args)
            .env("LLVM_PROFILE_FILE", llvm_profile_raw)
            // We must take the reference here
            // Otherwirse it gets dropped inside and the directory is deleted.
            .current_dir(&working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Spawning target")?;
        let mut stdin = process.stdin.take().expect("We set it to pipe");
        match stdin.write_all(input_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                // This probably means that the target is dead.
            }
            err => err.context("Writing to target stdin")?,
        }
        process.wait().context("Waiting for target")?;

        anyhow::ensure!(
            fs::exists(llvm_profile_raw).context("Checking LLVM raw profile data")?,
            "LLVM raw profile data not found."
        );
        Ok(())
    }

    fn run_llvm_cov(&self, llvm_profile_data: String) -> Result<lcov::Report, anyhow::Error> {
        let mut child = Command::new("llvm-cov")
            .args([
                "export",
                &self.executable.to_string_lossy(),
                "--instr-profile",
                &llvm_profile_data,
                "--format=lcov",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Spawn llvm-cov export to lcov")?;

        let stdout = child.stdout.take().expect("We set it to pipe");
        let lcov_reader = lcov::Reader::new(BufReader::new(stdout));
        let report = lcov::Report::from_reader(lcov_reader).context("Parsing lcov report")?;
        let llvm_cov_status = child.wait().context("Waiting llvm-cov")?;
        anyhow::ensure!(llvm_cov_status.success(), "llvm-cov failed");
        Ok(report)
    }
}
