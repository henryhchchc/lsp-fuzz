use std::{
    collections::HashMap,
    env::temp_dir,
    fs::File,
    hash::Hash,
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::Context;
use core_affinity::CoreId;
use itertools::Itertools;
use libafl::{
    corpus::{
        Corpus, CorpusId, HasTestcase, InMemoryCorpus, OnDiskCorpus, SchedulerTestcaseMetadata,
    },
    events::{EventFirer, SimpleEventManager},
    feedback_and_fast, feedback_or,
    feedbacks::{CrashFeedback, MaxMapFeedback, NewHashFeedback, TimeFeedback},
    monitors::SimpleMonitor,
    mutators::{StdScheduledMutator, Tokens},
    observers::{
        AsanBacktraceObserver, CanTrack, HitcountsMapObserver, MapObserver, Observer,
        StdMapObserver, TimeObserver,
    },
    schedulers::{
        powersched::{BaseSchedule, PowerSchedule},
        IndexesLenTimeMinimizerScheduler, Scheduler, StdWeightedScheduler,
    },
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::{HasCorpus, HasRand, StdState, UsesState},
    Evaluator, Fuzzer, HasMetadata, StdFuzzer,
};
use libafl_bolts::{
    current_nanos,
    rands::{Rand, StdRand},
    shmem::{ShMem, ShMemProvider, UnixShMemProvider},
    tuples::Handled,
    AsSliceMut, HasLen,
};
use lsp_fuzz::{
    afl,
    execution::LspExecutor,
    fuzz_target::FuzzBinaryInfo,
    lsp_input::{messages::message_mutations, LspInpuGenerator, LspInput, LspInputMutator},
    stages::CleanupWorkspaceDirs,
    text_document::{
        grammars::{DerivationFragments, DerivationGrammar, GrammarContext},
        text_document_mutations, GrammarContextLookup, Language,
    },
};

use nix::sys::signal::Signal;
use tracing::{info, warn};
use tuple_list::tuple_list;

use super::GlobalOptions;

const DEFAULT_COVERAGE_MAP_SIZE: usize = 65536;
const AFL_SHMEM_SIZE_ENV: &str = "AFL_MAP_SIZE";

/// Fuzz a Language Server Protocol (LSP) server.
#[derive(Debug, clap::Parser)]
pub(super) struct FuzzCommand {
    /// Directory containing seed inputs for the fuzzer.
    #[clap(long)]
    seeds_dir: Option<PathBuf>,

    /// Directory to store crash artifacts.
    #[clap(long)]
    crashes: Option<PathBuf>,

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
    #[clap(long, short, env = AFL_SHMEM_SIZE_ENV, default_value_t = DEFAULT_COVERAGE_MAP_SIZE)]
    coverage_map_size: usize,

    /// Shareed memory fuzzing.
    #[clap(long, short)]
    shared_memory_fuzzing: Option<usize>,

    /// Enable auto tokens.
    #[clap(long, env = "AFL_NO_AUTODICT")]
    no_auto_dict: bool,

    /// Number of seeds to generate if no seeds are provided.
    #[clap(long, default_value_t = 32)]
    generate_seeds: usize,

    /// Timeout runing the fuzz target in milliseconds.
    #[clap(long, short, default_value_t = 1200)]
    timeout: u64,

    /// Signal to send to terminate the child process.
    #[clap(long, short, default_value_t = Signal::SIGKILL)]
    kill_signal: Signal,

    /// Enable debugging for the child process.
    #[clap(long, env = "AFL_DEBUG", default_value_t = false)]
    debug_child: bool,

    /// Power schedule to use for fuzzing.
    #[clap(long, value_enum, default_value_t = BaseSchedule::EXPLORE)]
    power_schedule: BaseSchedule,

    /// Whether to cycle power schedules.
    #[clap(long, default_value_t = false)]
    cycle_power_schedule: bool,

    #[clap(long)]
    cpu_affinity: Option<usize>,

    #[clap(long, value_parser = parse_hash_map::<Language, PathBuf>)]
    language_fragments: HashMap<Language, PathBuf>,
}

fn parse_hash_map<K, V>(s: &str) -> Result<HashMap<K, V>, anyhow::Error>
where
    K: FromStr + Hash + Eq,
    V: FromStr,
    <K as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    <V as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    let mut result = HashMap::new();
    for pair in s.split(',') {
        let (key, value) = pair.split_once('=').context("Splitting key and value")?;
        let key = key.parse().context("Parsing key")?;
        let value = value.parse().context("Parsing value")?;
        result.insert(key, value);
    }
    Ok(result)
}

impl FuzzCommand {
    pub(super) fn run(self, global_options: GlobalOptions) -> Result<(), anyhow::Error> {
        info!("Analyzing fuzz target");
        let binary_info =
            FuzzBinaryInfo::from_binary(&self.lsp_executable).context("Analyzing fuzz traget")?;

        if binary_info.is_afl_instrumented {
            info!("Fuzz target is instrumented with AFL++");
        } else {
            warn!("The fuzz target is not instrumented with AFL++");
        }
        if binary_info.is_persistent_mode {
            info!("Persistent fuzzing detected.");
        }
        if binary_info.is_defer_fork_server {
            info!("Deferred fork server detected.");
        }
        if let Some(shm_size) = self.shared_memory_fuzzing {
            info!(shm_size, "Shared memory fuzzing is enabled.");
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
            .write_to_env(afl::SHMEM_ADDR_ENV)
            .context("Writing shared memory config to env")?;
        std::env::set_var(AFL_SHMEM_SIZE_ENV, self.coverage_map_size.to_string());

        // Create an observation channel using the signals map
        let shmem_observer = {
            let shmem_buf = shmem.as_slice_mut();
            // SAFETY: We never move the pirce of the shared memory.
            unsafe { StdMapObserver::new("edges", shmem_buf) }
        };

        let asan_observer = AsanBacktraceObserver::new("asan_stacktrace");

        let asan_handle = if binary_info.uses_address_sanitizer {
            info!("Fuzz target is compiled with AddressSanitizer.");
            Some(asan_observer.handle())
        } else {
            None
        };

        let edges_observer = HitcountsMapObserver::new(shmem_observer).track_indices();

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        let map_feedback = MaxMapFeedback::new(&edges_observer);
        let calibration_stage = CalibrationStage::new(&map_feedback);
        let mut feedback = feedback_or!(map_feedback, TimeFeedback::new(&time_observer));

        let mut objective = feedback_and_fast!(
            CrashFeedback::new(),
            NewHashFeedback::new(&asan_observer),
            // MaxMapFeedback::with_name("edges_objective", &edges_observer)
        );

        let corpus = InMemoryCorpus::<LspInput>::new();
        let crashes_dir = self
            .crashes
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("crashes_{}", current_nanos())));
        info!("Objectives will be saved at {}", &crashes_dir.display());
        let solution_corpus = OnDiskCorpus::new(crashes_dir).context("Creating solution corpus")?;

        let random_seed = global_options.random_seed.unwrap_or_else(current_nanos);
        let mut state = StdState::new(
            StdRand::with_seed(random_seed),
            corpus,
            solution_corpus,
            &mut feedback,
            &mut objective,
        )
        .context("Creating state")?;

        let mut tokens = if !self.no_auto_dict {
            Some(Tokens::new())
        } else {
            None
        };

        let scheduler = create_scheduler(
            &mut state,
            self.power_schedule,
            self.cycle_power_schedule,
            &edges_observer,
        );

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        let test_case_shm = self
            .shared_memory_fuzzing
            .map(|size| shmem_provider.new_shmem(size))
            .transpose()
            .context("Creating shared memory for test case passing")?;

        let temp_dir = temp_dir();
        let temp_dir_str = temp_dir
            .to_str()
            .context("temp_dir is not a vaild UTF-8 string")?;

        let mut fuzz_stages = {
            let mutation_stage = {
                let text_document_mutator = StdScheduledMutator::with_max_stack_pow(
                    text_document_mutations(&grammar_ctx),
                    2,
                );
                let messages_mutator =
                    StdScheduledMutator::with_max_stack_pow(message_mutations(), 4);
                let mutator = LspInputMutator::new(text_document_mutator, messages_mutator);
                StdPowerMutationalStage::new(mutator)
            };
            let cleanup_workspace_stage = CleanupWorkspaceDirs::new(temp_dir_str.to_owned(), 1000);
            tuple_list![calibration_stage, mutation_stage, cleanup_workspace_stage]
        };

        let mut executor = LspExecutor::new(
            &self.lsp_executable,
            self.target_args,
            None,
            Duration::from_millis(self.timeout).into(),
            self.debug_child,
            self.kill_signal,
            test_case_shm,
            binary_info.is_persistent_mode,
            binary_info.is_defer_fork_server,
            tokens.as_mut(),
            edges_observer,
            asan_handle,
            tuple_list![time_observer, asan_observer],
        )
        .context("Creating executor")?;

        let mut event_manager = {
            let monitor = SimpleMonitor::with_user_monitor(|it| info!("{}", it));
            SimpleEventManager::new(monitor)
        };

        if let Some(tokens) = tokens {
            state.add_metadata(tokens);
        }

        // In case the corpus is empty (on first run), reset
        initialize_corpus(
            self.seeds_dir,
            &mut state,
            &mut fuzzer,
            &mut executor,
            &mut event_manager,
            &grammar_ctx,
            self.generate_seeds,
        )?;

        state
            .corpus()
            .testcase_mut(CorpusId(0))
            .unwrap()
            .add_metadata(SchedulerTestcaseMetadata::new(0));

        if let Some(id) = self.cpu_affinity {
            let core_id = CoreId { id };
            if core_affinity::set_for_current(core_id) {
                info!("Set CPU affinity to core {}", id);
            } else {
                warn!("Failed to set CPU affinity to core {}", id);
            }
        }

        fuzzer
            .fuzz_loop(
                &mut fuzz_stages,
                &mut executor,
                &mut state,
                &mut event_manager,
            )
            .context("In fuzz loop")
    }

    fn create_grammar_context(&self) -> Result<GrammarContextLookup, anyhow::Error> {
        info!("Loading grammar files");
        let contexts: Vec<_> = self
            .language_fragments
            .iter()
            .map(|(&lang, frag_path)| -> Result<_, anyhow::Error> {
                let frags = load_derivation_fragments(frag_path)?;
                let grammar =
                    DerivationGrammar::from_tree_sitter_grammar_json(lang, lang.grammar_json())?;
                let grammar_ctx = GrammarContext::new(grammar, frags);
                Ok((lang, grammar_ctx))
            })
            .try_collect()?;
        let grammar_ctx = GrammarContextLookup::from_iter(contexts);
        Ok(grammar_ctx)
    }
}

fn load_derivation_fragments(path: &Path) -> Result<DerivationFragments, anyhow::Error> {
    let file = File::open(path).context("Opening derivation fragment")?;
    let reader = zstd::Decoder::new(BufReader::new(file))?;
    let result = serde_cbor::from_reader(reader).context("Deserializing derivation fragments")?;
    Ok(result)
}

fn create_scheduler<S, I, O, MO>(
    state: &mut S,
    power_schedule: BaseSchedule,
    cycle_power_schedule: bool,
    edges_observer: &O,
) -> impl Scheduler<I, S>
where
    I: HasLen,
    S: HasCorpus + HasMetadata + HasRand + HasTestcase,
    <S as HasCorpus>::Corpus: Corpus<Input = I>,
    O: Observer<I, S> + CanTrack + AsRef<MO>,
    MO: MapObserver,
{
    let power_schedule = PowerSchedule::new(power_schedule);
    let mut weighted_scheduler =
        StdWeightedScheduler::with_schedule(state, edges_observer, Some(power_schedule));
    if cycle_power_schedule {
        weighted_scheduler = weighted_scheduler.cycling_scheduler();
    }
    IndexesLenTimeMinimizerScheduler::new(edges_observer, weighted_scheduler)
}

fn initialize_corpus<E, Z, EM, C, R, SC>(
    seeds_dir: Option<PathBuf>,
    state: &mut StdState<LspInput, C, R, SC>,
    fuzzer: &mut Z,
    executor: &mut E,
    event_manager: &mut EM,
    grammar_context_lookup: &GrammarContextLookup,
    num_seeds: usize,
) -> Result<(), anyhow::Error>
where
    C: Corpus<Input = LspInput>,
    R: Rand,
    SC: Corpus<Input = LspInput>,
    Z: Evaluator<E, EM, State = StdState<LspInput, C, R, SC>>,
    E: UsesState<State = StdState<LspInput, C, R, SC>>,
    EM: EventFirer + UsesState<State = StdState<LspInput, C, R, SC>>,
{
    if state.must_load_initial_inputs() {
        if let Some(seeds_dir) = seeds_dir {
            state
                .load_initial_inputs(fuzzer, executor, event_manager, &[seeds_dir])
                .context("Loading seed inputs")?;
            info!(num_inputs = state.corpus().count(), "Seed inputs imported");
        } else {
            warn!("No seed inputs provided, starting from scratch");
            let mut generator = LspInpuGenerator::new(grammar_context_lookup);
            state
                .generate_initial_inputs_forced(
                    fuzzer,
                    executor,
                    &mut generator,
                    event_manager,
                    num_seeds,
                )
                .context("Generating initial input")?;
        }
    }
    Ok(())
}
