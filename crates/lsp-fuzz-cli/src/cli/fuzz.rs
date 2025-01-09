use std::{collections::HashMap, env::temp_dir, ops::Not, path::PathBuf, time::Duration};

use anyhow::{bail, Context};
use clap::builder::BoolishValueParser;
use core_affinity::CoreId;
use libafl::{
    corpus::{Corpus, InMemoryOnDiskCorpus},
    events::{EventFirer, SimpleEventManager},
    feedback_and_fast, feedback_or, feedback_or_fast,
    feedbacks::{ConstFeedback, CrashFeedback, MaxMapFeedback, NewHashFeedback, TimeFeedback},
    monitors::SimpleMonitor,
    mutators::{StdScheduledMutator, Tokens},
    observers::{
        AsanBacktraceObserver, CanTrack, HitcountsMapObserver, StdMapObserver, TimeObserver,
    },
    schedulers::{
        powersched::{BaseSchedule, PowerSchedule},
        IndexesLenTimeMinimizerScheduler, StdWeightedScheduler,
    },
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::{HasCorpus, StdState, UsesState},
    Evaluator, Fuzzer, HasMetadata, StdFuzzer,
};
use libafl_bolts::{
    current_nanos,
    fs::InputFile,
    rands::{Rand, StdRand},
    shmem::{ShMem, ShMemProvider, UnixShMemProvider},
    tuples::Handled,
    AsSliceMut, HasLen,
};
use lsp_fuzz::{
    afl,
    execution::{
        workspace_observer::WorkSpaceObserver, FuzzExecutionConfig, FuzzInput, FuzzTargetInfo,
        LspExecutor,
    },
    fuzz_target::{StaticTargetBinaryInfo, TargetBinaryInfo},
    lsp_input::{messages::message_mutations, LspInput, LspInputGenerator, LspInputMutator},
    stages::CleanupWorkspaceDirs,
    text_document::{
        text_document_mutations, token_novelty::TokenNoveltyFeedback, GrammarContextLookup,
        Language,
    },
};

use tracing::{error, info, warn};
use tuple_list::tuple_list;

use crate::{
    fuzzing::{ExecutorOptions, FuzzerStateDir},
    language_fragments::load_grammar_lookup,
};

use super::{parse_hash_map, GlobalOptions};

/// Fuzz a Language Server Protocol (LSP) server.
#[derive(Debug, clap::Parser)]
pub(super) struct FuzzCommand {
    /// Directory containing seed inputs for the fuzzer.
    #[clap(long)]
    seeds_dir: Option<PathBuf>,

    /// Directory containing the fuzzer states.
    #[clap(long)]
    state: FuzzerStateDir,

    /// Enable auto tokens.
    #[clap(long, env = "AFL_NO_AUTODICT", value_parser = BoolishValueParser::new())]
    no_auto_dict: bool,

    /// Number of seeds to generate if no seeds are provided.
    #[clap(long, default_value_t = 32)]
    generate_seeds: usize,

    #[clap(flatten)]
    execution: ExecutorOptions,

    /// The path to the temporary directory.
    #[clap(long, env = "AFL_TMPDIR")]
    temp_dir: Option<PathBuf>,

    /// Power schedule to use for fuzzing.
    #[clap(long, short, value_enum, default_value_t = BaseSchedule::FAST)]
    power_schedule: BaseSchedule,

    /// Whether to cycle power schedules.
    #[clap(long, env = "AFL_CYCLE_SCHEDULES", value_parser = BoolishValueParser::new())]
    cycle_power_schedule: bool,

    #[clap(long)]
    cpu_affinity: Option<usize>,

    #[clap(long, value_parser = parse_hash_map::<Language, PathBuf>)]
    language_fragments: HashMap<Language, PathBuf>,
}

impl FuzzCommand {
    pub(super) fn run(mut self, global_options: GlobalOptions) -> Result<(), anyhow::Error> {
        let mut shmem_provider =
            UnixShMemProvider::new().context("Creating shared memory provider")?;

        info!("Analyzing fuzz target");
        let binary_info = StaticTargetBinaryInfo::scan(&self.execution.lsp_executable)
            .context("Analyzing fuzz target")?;
        if binary_info.is_afl_instrumented {
            info!("Fuzz target is instrumented with AFL++");
        } else {
            warn!("The fuzz target is not instrumented with AFL++");
        }
        let binary_info = TargetBinaryInfo::detect(
            &self.execution.lsp_executable,
            &mut shmem_provider,
            binary_info,
        )?;

        if binary_info.is_persistent_mode {
            info!("Persistent fuzzing detected.");
        }
        if binary_info.is_defer_fork_server {
            info!("Deferred fork server detected.");
        }
        if let Some(shm_size) = self.execution.shared_memory_fuzzing {
            info!(shm_size, "Shared memory fuzzing is enabled.");
        }

        if binary_info.is_shmem_fuzzing && self.execution.shared_memory_fuzzing.is_none() {
            error!("Target requires shared memory fuzzing but the size of the shared memory is not specified.");
            bail!("Invalid configuration");
        }

        let Some(shmem_size) = binary_info
            .map_size
            .inspect(|it| info!("Detected coverage map size: {}", it))
            .or(self.execution.coverage_map_size)
        else {
            error!("Coverage map size could not be detected and is not specified.");
            bail!("Invalid configuration");
        };
        let mut cov_shmem = shmem_provider
            .new_shmem(shmem_size)
            .context("Creating shared memory")?;
        let cov_map_shmem_id = cov_shmem.id();

        info!("Loading grammar context");
        let grammar_ctx =
            load_grammar_lookup(&self.language_fragments).context("Creating grammar context")?;

        // Create an observation channel using the signals map
        let cov_map_observer = {
            let shmem_buf = cov_shmem.as_slice_mut();
            // SAFETY: We never move the pirce of the shared memory.
            unsafe { StdMapObserver::new("edges", shmem_buf) }
        };

        let asan_observer = AsanBacktraceObserver::new("asan_stacktrace");

        let asan_handle = binary_info.uses_address_sanitizer.then(|| {
            info!("Fuzz target is compiled with Address Sanitizer. Crash stack hashing will be enabled");
            asan_observer.handle()
        });

        let edges_observer = HitcountsMapObserver::new(cov_map_observer).track_indices();

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        let map_feedback = MaxMapFeedback::new(&edges_observer);
        let calibration_stage = CalibrationStage::new(&map_feedback);
        let novel_tokens = TokenNoveltyFeedback::new();
        let mut feedback = feedback_or!(
            map_feedback,
            novel_tokens,
            TimeFeedback::new(&time_observer)
        );

        let mut objective = feedback_and_fast!(
            CrashFeedback::new(),
            MaxMapFeedback::with_name("crash_cov", &edges_observer),
            feedback_or_fast!(
                ConstFeedback::new(binary_info.uses_address_sanitizer.not()),
                NewHashFeedback::new(&asan_observer)
            )
        );

        let corpus =
            InMemoryOnDiskCorpus::new(self.state.corpus_dir()).context("Creating corpus")?;

        let solutions = InMemoryOnDiskCorpus::new(self.state.solution_dir())
            .context("Creating solution corpus")?;

        let random_seed = global_options.random_seed.unwrap_or_else(current_nanos);
        let mut state = StdState::new(
            StdRand::with_seed(random_seed),
            corpus,
            solutions,
            &mut feedback,
            &mut objective,
        )
        .context("Creating state")?;

        let mut tokens = self.no_auto_dict.not().then(Tokens::new);

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

        let test_case_shm = self
            .execution
            .shared_memory_fuzzing
            .map(|size| shmem_provider.new_shmem(size))
            .transpose()
            .context("Creating shared memory for test case passing")?;

        let temp_dir = self.temp_dir.unwrap_or_else(temp_dir);
        let temp_dir_str = temp_dir
            .to_str()
            .context("temp_dir is not a valid UTF-8 string")?;

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

        let fuzz_input = if let Some(shm) = test_case_shm {
            FuzzInput::SharedMemory(shm)
        } else {
            let filename = format!("lsp-fuzz-input_{}", current_nanos());
            let input_file_path = temp_dir.join(filename);
            let input_file_path_str = input_file_path
                .to_str()
                .context("Invalid temp file path")?
                .to_owned();
            let input_file = InputFile::create(input_file_path)?;
            info!(path = %input_file.path.display(), "Created input file");

            if replace_at_args(&mut self.execution.target_args, input_file_path_str) {
                FuzzInput::File(input_file)
            } else {
                FuzzInput::Stdin(input_file)
            }
        };

        let target_info = FuzzTargetInfo {
            path: self.execution.lsp_executable,
            args: self.execution.target_args,
            persistent_fuzzing: binary_info.is_persistent_mode,
            defer_fork_server: binary_info.is_defer_fork_server,
            crash_exit_code: self.execution.crash_exit_code,
            timeout: Duration::from_millis(self.execution.timeout).into(),
            kill_signal: self.execution.kill_signal,
        };

        let workspace_observer = WorkSpaceObserver;

        let exec_config = FuzzExecutionConfig {
            debug_child: self.execution.debug_child,
            debug_afl: self.execution.debug_afl,
            fuzz_input,
            auto_tokens: tokens.as_mut(),
            coverage_map_info: Some((cov_map_shmem_id, edges_observer.as_ref().len())),
            map_observer: edges_observer,
            asan_observer_handle: asan_handle,
            other_observers: tuple_list![workspace_observer, asan_observer, time_observer],
        };

        let mut executor =
            LspExecutor::start(target_info, exec_config).context("Starting executor")?;

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

        if let Some(id) = self.cpu_affinity {
            let core_id = CoreId { id };
            if core_affinity::set_for_current(core_id) {
                info!("Set CPU affinity to core {id}");
            } else {
                warn!("Failed to set CPU affinity to core {id}");
            }
        }

        let fuzz_result = fuzzer.fuzz_loop(
            &mut fuzz_stages,
            &mut executor,
            &mut state,
            &mut event_manager,
        );

        match fuzz_result {
            Ok(()) => unreachable!("The fuzz loop will never exit with Ok"),
            Err(libafl::Error::ShuttingDown) => {
                info!(
                    "Stop requested by user. {} will now exit.",
                    crate::PROGRAM_NAME
                );
                Ok(())
            }
            err @ Err(_) => err.context("In fuzz loop"),
        }
    }
}

fn initialize_corpus<E, Z, EM, R, C, SC>(
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
    Z: Evaluator<E, EM, LspInput, StdState<LspInput, C, R, SC>>,
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
            let mut generator = LspInputGenerator::new(grammar_context_lookup);
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

pub fn replace_at_args(target_args: &mut [String], input_file_path_str: String) -> bool {
    let mut replaced = false;
    target_args
        .iter_mut()
        .filter(|it| *it == afl::ARG_FILE_PLACE_HOLDER)
        .for_each(|input_file| {
            *input_file = input_file_path_str.clone();
            replaced = true;
        });
    replaced
}
