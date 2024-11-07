use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use core_affinity::CoreId;
use libafl::{
    corpus::{Corpus, InMemoryCorpus, OnDiskCorpus},
    events::SimpleEventManager,
    feedback_and_fast, feedback_or,
    feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback},
    generators::Generator,
    monitors::SimpleMonitor,
    mutators::{StdScheduledMutator, Tokens},
    observers::{CanTrack, HitcountsMapObserver, StdMapObserver, TimeObserver},
    schedulers::{
        powersched::{BaseSchedule, PowerSchedule},
        IndexesLenTimeMinimizerScheduler, StdWeightedScheduler,
    },
    stages::{
        CalibrationStage, MapEqualityFactory, StdPowerMutationalStage, StdTMinMutationalStage,
    },
    state::{HasCorpus, StdState},
    Evaluator, Fuzzer, HasMetadata, StdFuzzer,
};
use libafl_bolts::{
    current_nanos,
    rands::StdRand,
    shmem::{ShMem, ShMemProvider, UnixShMemProvider},
    AsSliceMut,
};
use lsp_fuzz::{
    execution::LspExecutor,
    lsp_input::{LspInpuGenerator, LspInput, LspInputMutator},
    stages::{CleanupWorkspaceDirs, CoverageStage},
    text_document::{
        grammars::{DerivationFragments, DerivationGrammar, GrammarContext, C_GRAMMAR_JSON},
        text_document_mutations, text_document_reductions, GrammarContextLookup, Language,
    },
};
use nix::sys::signal::Signal;
use tracing::{info, warn};
use tuple_list::tuple_list;

use super::GlobalOptions;

const DEFAULT_COVERAGE_MAP_SIZE: usize = 65536;
const AFL_SHMEM_ADDR_ENV: &str = "__AFL_SHM_ID";
const AFL_SHMEM_SIZE_ENV: &str = "AFL_MAP_SIZE";

/// Fuzz a Language Server Protocol (LSP) server.
#[derive(Debug, clap::Parser)]
pub(super) struct FuzzCommand {
    /// Directory containing seed inputs for the fuzzer.
    #[clap(long)]
    seeds_dir: Option<PathBuf>,

    /// Directory to store crash artifacts.
    #[clap(long, default_value = "crashes")]
    crashes: PathBuf,

    /// Working directory for the Language Server Protocol (LSP).
    #[clap(long)]
    lsp_work_dir: Option<PathBuf>,

    /// Path to the LSP executable.
    #[clap(long)]
    lsp_executable: PathBuf,

    /// Arguments to pass to the child process.
    #[clap(long)]
    target_args: Vec<String>,

    /// Size of the coverage map.
    #[clap(long, short, default_value_t = DEFAULT_COVERAGE_MAP_SIZE)]
    coverage_map_size: usize,

    /// Timeout runing the fuzz target in milliseconds.
    #[clap(long, short, default_value_t = 1200)]
    timeout: u64,

    /// Signal to send to terminate the child process.
    #[clap(long, short, default_value_t = Signal::SIGKILL)]
    kill_signal: Signal,

    /// Enable debugging for the child process.
    #[clap(long, default_value_t = false)]
    debug_child: bool,

    /// Power schedule to use for fuzzing.
    #[clap(long, value_enum, default_value_t = BaseSchedule::FAST)]
    power_schedule: BaseSchedule,

    /// Whether to cycle power schedules.
    #[clap(long, default_value_t = false)]
    cycle_power_schedule: bool,

    #[clap(long)]
    cpu_affinity: Option<usize>,

    #[clap(long)]
    c_derivation_fragment: PathBuf,
}

impl FuzzCommand {
    pub(super) fn run(self, global_options: GlobalOptions) -> Result<(), anyhow::Error> {
        if let Some(id) = self.cpu_affinity {
            let core_id = CoreId { id };
            if core_affinity::set_for_current(core_id) {
                info!("Set CPU affinity to core {}", id);
            } else {
                warn!("Failed to set CPU affinity to core {}", id);
            }
        }

        let grammar_ctx = self
            .create_grammar_context()
            .context("Creating grammar context")?;

        let mut shmem_provider =
            UnixShMemProvider::new().context("Creating shared memory provider")?;

        // The coverage map shared between observer and executor
        let mut shmem = shmem_provider
            .new_shmem(self.coverage_map_size)
            .context("Creating shared memory")?;
        // let the forkserver know the shmid
        shmem
            .write_to_env(AFL_SHMEM_ADDR_ENV)
            .context("Writing shared memory config to env")?;
        std::env::set_var(AFL_SHMEM_SIZE_ENV, self.coverage_map_size.to_string());

        // Create an observation channel using the signals map
        let shmem_observer = {
            let shmem_buf = shmem.as_slice_mut();
            // SAFETY: We never move the pirce of the shared memory.
            unsafe { StdMapObserver::differential("edges", shmem_buf) }
        };

        let edges_observer = HitcountsMapObserver::new(shmem_observer).track_indices();
        let coverage_stage = CoverageStage::new(&edges_observer);

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        // Feedback to rate the interestingness of an input
        // This one is composed by two Feedbacks in OR
        // New maximization map feedback linked to the edges observer and the feedback state
        let map_feedback = MaxMapFeedback::new(&edges_observer);
        let calibration_stage = CalibrationStage::new(&map_feedback);
        let mut feedback = feedback_or!(map_feedback, TimeFeedback::new(&time_observer));
        let min_feedback_factory = MapEqualityFactory::new(&edges_observer);

        // A feedback to choose if an input is a solution or not
        // We want to do the same crash deduplication that AFL does
        let mut objective = feedback_and_fast!(
            // Must be a crash
            CrashFeedback::new(),
            // Take it only if trigger new coverage over crashes
            // Uses `with_name` to create a different history from the `MaxMapFeedback` in `feedback` above
            MaxMapFeedback::with_name("mapfeedback_metadata_objective", &edges_observer)
        );

        let corpus = InMemoryCorpus::<LspInput>::new();
        let solution_corpus =
            OnDiskCorpus::new(self.crashes).context("Creating solution corpus")?;

        let random_seed = global_options.random_seed.unwrap_or_else(current_nanos);
        let mut state = StdState::new(
            StdRand::with_seed(random_seed),
            corpus,
            solution_corpus,
            &mut feedback,
            &mut objective,
        )
        .context("Creating state")?;

        let mut tokens = Tokens::new();

        let scheduler = {
            let power_schedule = PowerSchedule::new(self.power_schedule);
            let mut weighted_scheduler = StdWeightedScheduler::with_schedule(
                &mut state,
                &edges_observer,
                Some(power_schedule),
            );
            if self.cycle_power_schedule {
                weighted_scheduler = weighted_scheduler.cycling_scheduler();
            }
            IndexesLenTimeMinimizerScheduler::new(&edges_observer, weighted_scheduler)
        };

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        let mut executor = LspExecutor::new(
            &self.lsp_executable,
            self.target_args,
            None,
            Duration::from_millis(self.timeout).into(),
            self.debug_child,
            self.kill_signal,
            Some(&mut tokens),
            edges_observer,
            tuple_list!(time_observer),
        )
        .context("Creating executor")?;

        let mut event_manager = {
            let monitor = SimpleMonitor::new(|s| info!("{s}"));
            SimpleEventManager::new(monitor)
        };

        // In case the corpus is empty (on first run), reset
        if state.must_load_initial_inputs() {
            if let Some(seeds_dir) = self.seeds_dir {
                state
                    .load_initial_inputs(
                        &mut fuzzer,
                        &mut executor,
                        &mut event_manager,
                        &[seeds_dir],
                    )
                    .context("Loading seed inputs")?;
                info!(num_inputs = state.corpus().count(), "Seed inputs imported");
            } else {
                warn!("No seed inputs provided, starting from scratch");
                let mut generator = LspInpuGenerator;
                let initial_input = generator
                    .generate(&mut state)
                    .context("Generating initial input")?;
                fuzzer
                    .add_input(&mut state, &mut executor, &mut event_manager, initial_input)
                    .context("Adding initial input")?;
            }
        }

        state.add_metadata(tokens);

        let text_document_mutator =
            StdScheduledMutator::with_max_stack_pow(text_document_mutations(&grammar_ctx), 2);
        let mutator = LspInputMutator::new(text_document_mutator);
        let mutation_stage = StdPowerMutationalStage::new(mutator);

        let text_document_reducer =
            StdScheduledMutator::with_max_stack_pow(text_document_reductions(&grammar_ctx), 2);
        let reducer = LspInputMutator::new(text_document_reducer);
        let minimization_stage = StdTMinMutationalStage::new(reducer, min_feedback_factory, 128);

        let cleanup_workspace_stage = CleanupWorkspaceDirs::new();

        let mut stages = tuple_list![
            minimization_stage,
            calibration_stage,
            mutation_stage,
            coverage_stage,
            cleanup_workspace_stage
        ];

        fuzzer
            .fuzz_loop(&mut stages, &mut executor, &mut state, &mut event_manager)
            .context("In fuzzloop")
    }

    fn create_grammar_context(&self) -> Result<GrammarContextLookup, anyhow::Error> {
        let derivation_fragement = load_c_derivation_fragment(&self.c_derivation_fragment)
            .context("Loading C derivation fragements")?;
        let c_grammar =
            DerivationGrammar::from_tree_sitter_grammar_json(Language::C, C_GRAMMAR_JSON)
                .context("Constructing C grammar")?;
        let c_grammar_ctx = GrammarContext::new(c_grammar, derivation_fragement);
        let grammar_ctx = GrammarContextLookup::from_iter([(Language::C, c_grammar_ctx)]);
        Ok(grammar_ctx)
    }
}

fn load_c_derivation_fragment(path: &Path) -> Result<DerivationFragments, anyhow::Error> {
    info!(file = %path.display(), "Loading C derivation fragements");
    let file = File::open(path).context("Opening c derivation fragment")?;
    let reader = zstd::Decoder::new(BufReader::new(file))?;
    let result = serde_cbor::from_reader(reader).context("Deserializing derivation fragments")?;
    info!("C derivation fragments loaded");
    Ok(result)
}
