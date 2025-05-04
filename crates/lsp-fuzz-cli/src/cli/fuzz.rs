use std::{collections::HashMap, ops::Not, path::PathBuf, sync::mpsc, time::Duration};

use anyhow::{Context, bail};
use clap::builder::BoolishValueParser;
use core_affinity::CoreId;
use libafl::{
    Evaluator, Fuzzer, HasMetadata, HasNamedMetadata, NopInputFilter, StdFuzzerBuilder,
    corpus::{Corpus, HasCurrentCorpusId, InMemoryOnDiskCorpus, ondisk::OnDiskMetadataFormat},
    events::{EventFirer, SimpleEventManager},
    executors::{Executor, HasObservers},
    feedback_and_fast, feedback_or, feedback_or_fast,
    feedbacks::{ConstFeedback, CrashFeedback, MaxMapFeedback, NewHashFeedback, TimeFeedback},
    monitors::SimpleMonitor,
    mutators::HavocScheduledMutator,
    observers::{
        AsanBacktraceObserver, CanTrack, HitcountsMapObserver, StdMapObserver, TimeObserver,
    },
    schedulers::{
        IndexesLenTimeMinimizerScheduler, StdWeightedScheduler,
        powersched::{BaseSchedule, PowerSchedule},
    },
    stages::{CalibrationStage, Restartable, Stage, StdPowerMutationalStage},
    state::{
        HasCorpus, HasExecutions, HasMaxSize, HasRand, HasSolutions, MaybeHasClientPerfMonitor,
        StdState,
    },
};
use libafl_bolts::{
    AsSliceMut, HasLen,
    rands::{Rand, StdRand},
    shmem::{ShMem, ShMemProvider, StdShMemProvider},
};
use lsp_fuzz::{
    execution::{
        FuzzExecutionConfig, FuzzInput, FuzzTargetInfo, LspExecutor,
        workspace_observer::WorkspaceObserver,
    },
    fuzz_target::{self, StaticTargetBinaryInfo},
    lsp_input::{
        LspInput, LspInputBytesConverter, LspInputGenerator, LspInputMutator,
        messages::message_mutations,
    },
    stages::{StopOnReceived, TimeoutStopStage},
    text_document::{
        generation::GrammarContextLookup, text_document_mutations,
        token_novelty::TokenNoveltyFeedback,
    },
    utf8::UTF8Tokens,
};
use lsp_fuzz_grammars::Language;
use tracing::{info, warn};
use tuple_list::tuple_list;

use super::{GlobalOptions, parse_hash_map};
use crate::{
    fuzzing::{ExecutorOptions, FuzzerStateDir},
    language_fragments::load_grammar_lookup,
};

/// Fuzz a Language Server Protocol (LSP) server.
#[derive(Debug, clap::Parser)]
pub(super) struct FuzzCommand {
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

    /// Bind the fuzzer to a specific CPU core.
    #[clap(long)]
    cpu_affinity: Option<usize>,

    /// Stop fuzzing after a certain number of hours.
    #[clap(long)]
    time_budget: u64,

    #[clap(long)]
    no_asan: bool,

    #[clap(long, value_parser = parse_hash_map::<Language, PathBuf>)]
    language_fragments: HashMap<Language, PathBuf>,
}

impl FuzzCommand {
    pub(super) fn run(self, global_options: GlobalOptions) -> Result<(), anyhow::Error> {
        let mut shmem_provider =
            StdShMemProvider::new().context("Creating shared memory provider")?;

        info!("Analyzing fuzz target");
        let binary_info = StaticTargetBinaryInfo::scan(&self.execution.lsp_executable)
            .context("Analyzing fuzz target")?;
        if !binary_info.is_afl_instrumented {
            bail!("The fuzz target is not instrumented with AFL++");
        }
        if binary_info.is_persistent_mode {
            info!("Persistent fuzzing detected.");
        }
        if binary_info.is_defer_fork_server {
            info!("Deferred fork server detected.");
        }
        if binary_info.uses_address_sanitizer {
            info!("Fuzz target is compiled with Address Sanitizer.");
        }

        let map_size = fuzz_target::dump_map_size(&self.execution.lsp_executable)
            .context("Dumping map size")?;

        info!("Detected coverage map size: {}", map_size);
        let mut coverage_shmem = shmem_provider
            .new_shmem(map_size)
            .context("Creating shared memory")?;
        let coverage_map_shmem_id = coverage_shmem.id();

        info!("Loading grammar context");
        let grammar_ctx =
            load_grammar_lookup(&self.language_fragments).context("Creating grammar context")?;

        let coverage_map_observer = {
            let shmem_buf = coverage_shmem.as_slice_mut();
            // SAFETY: We never move the piece of the shared memory.
            unsafe { StdMapObserver::new("edges", shmem_buf) }
        };

        let asan_observer = AsanBacktraceObserver::new("asan_stacktrace");

        let asan_enabled = binary_info.uses_address_sanitizer && self.no_asan.not();
        let edges_observer = HitcountsMapObserver::new(coverage_map_observer).track_indices();

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        let map_feedback = MaxMapFeedback::new(&edges_observer);
        let calibration_stage = CalibrationStage::new(&map_feedback);
        let novel_tokens = TokenNoveltyFeedback::new(20);
        let mut feedback = feedback_or!(
            map_feedback,
            novel_tokens,
            TimeFeedback::new(&time_observer)
        );

        let mut objective = feedback_and_fast!(
            CrashFeedback::new(),
            MaxMapFeedback::with_name("crash_edges", &edges_observer),
            feedback_or_fast!(
                ConstFeedback::new(!asan_enabled),
                NewHashFeedback::new(&asan_observer)
            )
        );

        let corpus =
            InMemoryOnDiskCorpus::no_meta(self.state.corpus_dir()).context("Creating corpus")?;

        let solutions = InMemoryOnDiskCorpus::with_meta_format(
            self.state.solution_dir(),
            Some(OnDiskMetadataFormat::JsonGzip),
        )
        .context("Creating solution corpus")?;

        let random_seed = global_options
            .random_seed
            .unwrap_or_else(libafl_bolts::current_nanos);
        let rand = StdRand::with_seed(random_seed);
        let mut state = StdState::new(rand, corpus, solutions, &mut feedback, &mut objective)
            .context("Creating state")?;

        let mut tokens = self.no_auto_dict.not().then(UTF8Tokens::new);

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
        let temp_dir = self.temp_dir.unwrap_or_else(std::env::temp_dir);

        // A fuzzer with feedback and a corpus scheduler
        let mut fuzzer = StdFuzzerBuilder::new()
            .input_filter(NopInputFilter)
            .bytes_converter(LspInputBytesConverter::new(temp_dir.clone()))
            .build(scheduler, feedback, objective)
            .context("Building fuzzer")?;

        let mut fuzz_stages = {
            let mutation_stage = mutation_stage(&mut state, &grammar_ctx)?;
            let trigger_stop = trigger_stop_stage()?;
            let timeout_stop = TimeoutStopStage::new(Duration::from_hours(self.time_budget));
            tuple_list![
                calibration_stage,
                mutation_stage,
                timeout_stop,
                trigger_stop,
            ]
        };

        let asan_observer = asan_enabled.then_some(asan_observer);
        if asan_observer.is_some() {
            info!("Crash stack hashing will be enabled");
        }
        let mut executor = {
            let execution_config = self.execution;
            const INPUT_SHM_SIZE: usize = 15 * 1024 * 1024 * 1024;
            let test_case_shmem = shmem_provider
                .new_shmem(INPUT_SHM_SIZE)
                .context("Creating shared memory for test case passing")?;
            let fuzz_input = FuzzInput::SharedMemory(test_case_shmem);
            let target_info = FuzzTargetInfo {
                path: execution_config.lsp_executable,
                args: execution_config.target_args,
                persistent_fuzzing: binary_info.is_persistent_mode,
                defer_fork_server: binary_info.is_defer_fork_server,
                crash_exit_code: execution_config.crash_exit_code,
                timeout: Duration::from_millis(execution_config.exec_timeout).into(),
                kill_signal: execution_config.kill_signal,
                env: execution_config.target_env,
            };
            let workspace_observer = WorkspaceObserver::new(temp_dir);
            let exec_config = FuzzExecutionConfig {
                debug_child: execution_config.debug_child,
                debug_afl: execution_config.debug_afl,
                fuzz_input,
                auto_tokens: tokens.as_mut(),
                coverage_shm_info: Some((coverage_map_shmem_id, edges_observer.as_ref().len())),
                map_observer: edges_observer,
                asan_observer,
                other_observers: tuple_list![workspace_observer, time_observer],
            };
            LspExecutor::start(target_info, exec_config).context("Starting executor")?
        };

        if let Some(tokens) = tokens {
            info!("Extracted {} UTF-8 token(s) from the target.", tokens.len());
            state.add_metadata(tokens);
        }

        let mut event_manager = {
            let monitor = SimpleMonitor::with_user_monitor(|it| info!("{}", it));
            SimpleEventManager::new(monitor)
        };

        // In case the corpus is empty (on first run), reset
        initialize_corpus(
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

fn trigger_stop_stage<I>() -> Result<StopOnReceived<I>, anyhow::Error> {
    let (tx, rx) = mpsc::channel();
    let mut is_control_c_pressed = false;
    ctrlc::try_set_handler(move || {
        if is_control_c_pressed {
            info!("Control-C pressed again. Exiting immediately.");
            const EXIT_CODE: i32 = 128 + (nix::sys::signal::SIGINT as i32);
            std::process::exit(EXIT_CODE);
        }
        is_control_c_pressed = true;
        info!("Control-C pressed. The fuzzer will stop after this cycle.");
        tx.send(()).expect("Failed to send stop signal");
    })
    .context("Setting Control-C handler")?;

    Ok(StopOnReceived::new(rx))
}

fn mutation_stage<'g, Exec, EventMgr, State, Fuzzer>(
    _state: &mut State,
    grammar_ctx: &'g GrammarContextLookup,
) -> Result<
    impl Stage<Exec, EventMgr, State, Fuzzer>
    + Restartable<State>
    + use<'g, Exec, EventMgr, State, Fuzzer>,
    libafl::Error,
>
where
    State: HasRand
        + HasMaxSize
        + HasMetadata
        + HasCorpus<LspInput>
        + HasSolutions<LspInput>
        + HasCurrentCorpusId
        + HasNamedMetadata
        + HasExecutions
        + MaybeHasClientPerfMonitor + 'static,
    Fuzzer: Evaluator<Exec, EventMgr, LspInput, State>,
    Exec: Executor<EventMgr, LspInput, State, Fuzzer> + HasObservers,
{
    let text_document_mutator =
        HavocScheduledMutator::with_max_stack_pow(text_document_mutations(grammar_ctx), 4);
    let messages_mutator = HavocScheduledMutator::with_max_stack_pow(message_mutations(), 6);
    let mutator = LspInputMutator::new(text_document_mutator, messages_mutator);
    Ok(StdPowerMutationalStage::new(mutator))
}

fn initialize_corpus<E, Z, EM, R, C, SC>(
    state: &mut StdState<C, LspInput, R, SC>,
    fuzzer: &mut Z,
    executor: &mut E,
    event_manager: &mut EM,
    grammar_context_lookup: &GrammarContextLookup,
    num_seeds: usize,
) -> Result<(), anyhow::Error>
where
    R: Rand,
    C: Corpus<LspInput>,
    SC: Corpus<LspInput>,
    Z: Evaluator<E, EM, LspInput, StdState<C, LspInput, R, SC>>,
    EM: EventFirer<LspInput, StdState<C, LspInput, R, SC>>,
{
    if state.must_load_initial_inputs() {
        info!("Generating seeds");
        let mut generator = LspInputGenerator::new(grammar_context_lookup);
        state
            .generate_initial_inputs(fuzzer, executor, &mut generator, event_manager, num_seeds)
            .context("Generating initial input")?;
        info!(seeds = %state.corpus().count(), "Seed generation completed");
    }
    Ok(())
}
