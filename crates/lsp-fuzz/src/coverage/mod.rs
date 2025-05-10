use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Context;
use libafl::inputs::InputToBytes;
use tempfile::TempDir;

const RAW_COVERAGE_DATA_FILE_EXT: &str = "profraw";
const MERGED_COVERAGE_DATA_FILE_EXT: &str = "profdata";

#[derive(Debug)]
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
    pub fn input_coverage_measure<I, IB>(
        &self,
        input: &I,
        converter: &mut IB,
        coverage_data_path: &Path,
    ) -> anyhow::Result<()>
    where
        IB: InputToBytes<I>,
    {
        let temp_dir = TempDir::new().context("Creating tempdir")?;
        let bytes = converter.to_bytes(input);

        let llvm_profile_raw = format!(
            "{}/coverage.{}",
            temp_dir.path().display(),
            RAW_COVERAGE_DATA_FILE_EXT
        );
        let working_dir = temp_dir.path().join("working_dir");
        fs::create_dir_all(&working_dir).context("Creating working dir")?;
        let mut process = Command::new(&self.executable)
            .args(&self.args)
            .current_dir(working_dir)
            .env("LLVM_PROFILE_FILE", &llvm_profile_raw)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Spawning target")?;

        let mut stdin = process.stdin.take().expect("We set it to pipe");
        match stdin.write_all(&bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                // This probably means that the target is dead.
            }
            err => err.context("Writing to target stdin")?,
        }
        process.wait().context("Waiting for target")?;

        let llvm_profile_data = format!(
            "{}/coverage.{}",
            temp_dir.path().display(),
            MERGED_COVERAGE_DATA_FILE_EXT
        );
        Command::new("llvm-profdata")
            .args(["merge", "-sparse"])
            .arg(llvm_profile_raw)
            .arg("-o")
            .arg(&llvm_profile_data)
            .status()
            .context("Running llvm-profdata")?;

        // Export coverage data to lcov format
        let lcov_output_path = coverage_data_path.with_extension("lcov");
        let lcov_file = File::create(&lcov_output_path).context("Creating lcov output file")?;
        Command::new("llvm-cov")
            .args([
                "export",
                &self.executable.to_string_lossy(),
                "--instr-profile",
                &llvm_profile_data,
                "--format=lcov",
            ])
            .stdout(lcov_file)
            .status()
            .context("Running llvm-cov export to lcov")?;

        Ok(())
    }
}
