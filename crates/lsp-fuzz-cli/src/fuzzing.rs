use std::{collections::HashMap, fs, io, path::PathBuf};

use clap::builder::BoolishValueParser;
use nix::sys::signal::Signal;

use crate::cli::{parse_hash_map, parse_size};

pub mod common;

#[derive(Debug, Clone)]
pub struct FuzzerStateDir(PathBuf);

impl<P: Into<PathBuf>> From<P> for FuzzerStateDir {
    fn from(value: P) -> Self {
        Self(value.into())
    }
}

impl FuzzerStateDir {
    pub fn create(&self) -> io::Result<()> {
        fs::create_dir_all(&self.0)
    }

    pub fn corpus_dir(&self) -> PathBuf {
        self.0.join("corpus")
    }

    pub fn solution_dir(&self) -> PathBuf {
        self.0.join("solutions")
    }

    pub fn stats_file(&self) -> PathBuf {
        self.0.join("stats")
    }
}

#[derive(Debug, clap::Parser)]
pub struct ExecutorOptions {
    /// Path to the LSP executable.
    #[clap(long)]
    pub lsp_executable: PathBuf,

    /// Arguments to pass to the child process.
    #[clap(long)]
    pub target_args: Vec<String>,

    /// Environment variables to pass to the child process.
    /// Format: KEY=VALUE
    #[clap(long, value_parser = parse_hash_map::<String, String>, default_value = "")]
    pub target_env: HashMap<String, String>,

    /// Size of the coverage map.
    #[clap(long, short, env = "AFL_MAP_SIZE", value_parser = parse_size)]
    pub coverage_map_size: Option<usize>,

    /// Exit code that indicates a crash.
    #[clap(long, env = "AFL_CRASH_EXITCODE")]
    pub crash_exit_code: Option<i8>,

    /// Timeout running the fuzz target in milliseconds.
    #[clap(long, short, default_value_t = 1200)]
    pub exec_timeout: u64,

    /// Signal to send to terminate the child process.
    #[clap(long, short, env = "AFL_KILL_SIGNAL", default_value_t = Signal::SIGKILL)]
    pub kill_signal: Signal,

    /// Enable debugging for the child process.
    #[clap(long, env = "AFL_DEBUG_CHILD", value_parser = BoolishValueParser::new())]
    pub debug_child: bool,

    /// Enable debugging for AFL itself.
    #[clap(long, env = "AFL_DEBUG", value_parser = BoolishValueParser::new())]
    pub debug_afl: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum AblationMode {
    Full,
    NoServerFeedback,
    NoContextAwareness,
    AllOff,
}
