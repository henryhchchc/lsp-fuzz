use std::{
    collections::HashMap, fs::OpenOptions, io::BufWriter, ops::Not, path::PathBuf, time::Duration,
};

use anyhow::Context;
use clap::builder::BoolishValueParser;
use libafl::{
    Fuzzer, NopInputFilter, StdFuzzerBuilder,
    corpus::Corpus,
    events::SimpleEventManager,
    feedback_or,
    feedbacks::{ConstFeedback, FastAndFeedback, MaxMapFeedback, TimeFeedback},
    monitors::SimpleMonitor,
    mutators::HavocScheduledMutator,
    observers::{
        AsanBacktraceObserver, CanTrack, HitcountsMapObserver, StdMapObserver, TimeObserver,
    },
    schedulers::powersched::BaseSchedule,
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::{HasCorpus, StdState},
};
use libafl_bolts::{
    AsSliceMut, HasLen,
    rands::StdRand,
    shmem::{ShMem, ShMemProvider, StdShMemProvider},
};
use lsp_fuzz::{
    corpus::TestCaseFileNameFeedback,
    execution::{
        FuzzExecutionConfig, FuzzInput, LspExecutor, workspace_observer::WorkspaceObserver,
    },
    fuzz_target,
    lsp::GeneratorsConfig,
    lsp_input::{
        LspInputBytesConverter, LspInputGenerator, LspInputMutator, messages::message_mutations,
        ops_curiosity::CuriosityFeedback,
    },
    stages::{StatsStage, TimeoutStopStage},
    text_document::text_document_mutations,
    utf8::UTF8Tokens,
};
use lsp_fuzz_grammars::Language;
use tracing::info;
use tuple_list::tuple_list;

use super::{GlobalOptions, parse_hash_map};
use crate::{
    fuzzing::{
        AblationMode, ExecutorOptions, FuzzerStateDir,
        common::{self},
    },
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

    #[clap(long, value_enum, default_value_t = AblationMode::Full)]
    ablation_mode: AblationMode,

    #[clap(long, value_parser = parse_hash_map::<Language, PathBuf>)]
    language_fragments: HashMap<Language, PathBuf>,
}

impl FuzzCommand {
    pub(super) fn run(self, global_options: GlobalOptions) -> Result<(), anyhow::Error> {
        self.state.create().context("Crating state dir")?;
        let mut shmem_provider =
            StdShMemProvider::new().context("Creating shared memory provider")?;

        let binary_info = common::analyze_fuzz_target(&self.execution.lsp_executable)?;

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
        let cov_observer = HitcountsMapObserver::new(coverage_map_observer).track_indices();

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        let map_feedback = MaxMapFeedback::new(&cov_observer);
        let calibration_stage = CalibrationStage::new(&map_feedback);
        let curiosity_gate = match self.ablation_mode {
            AblationMode::Full | AblationMode::NoErrorInjection => ConstFeedback::True,
            AblationMode::NoCuriosity => ConstFeedback::False,
        };
        let curiosity_feedback = FastAndFeedback::new(curiosity_gate, CuriosityFeedback::new(20));
        let stats_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.state.stats_file())
            .context("Creating stats file")?;
        let stats_writer = BufWriter::new(stats_file);
        let stats_stage = StatsStage::new(stats_writer, &map_feedback);

        let mut feedback = feedback_or!(
            map_feedback,
            curiosity_feedback,
            TestCaseFileNameFeedback::new(),
            TimeFeedback::new(&time_observer)
        );

        let mut objective = common::objective(asan_enabled, &asan_observer);

        let (corpus, solutions) =
            common::create_corpus(&self.state.corpus_dir(), &self.state.solution_dir())
                .context("Creating corpus")?;

        let random_seed = global_options
            .random_seed
            .unwrap_or_else(libafl_bolts::current_nanos);
        let rand = StdRand::with_seed(random_seed);
        let mut state = StdState::new(rand, corpus, solutions, &mut feedback, &mut objective)
            .context("Creating state")?;

        let mut tokens = self.no_auto_dict.not().then(UTF8Tokens::new);

        let scheduler = common::scheduler(
            &mut state,
            &cov_observer,
            self.power_schedule,
            self.cycle_power_schedule,
        );
        let temp_dir = self.temp_dir.unwrap_or_else(std::env::temp_dir);

        // A fuzzer with feedback and a corpus scheduler
        let mut fuzzer = StdFuzzerBuilder::new()
            .input_filter(NopInputFilter)
            .bytes_converter(LspInputBytesConverter::new(temp_dir.clone()))
            .build(scheduler, feedback, objective)
            .context("Building fuzzer")?;

        let mut fuzz_stages = {
            let mutation_stage = {
                let generators_config = match self.ablation_mode {
                    AblationMode::Full | AblationMode::NoCuriosity => GeneratorsConfig::full(),
                    AblationMode::NoErrorInjection => GeneratorsConfig::no_error_injection(),
                };
                let text_document_mutator = HavocScheduledMutator::with_max_stack_pow(
                    text_document_mutations(&grammar_ctx, &generators_config),
                    4,
                );
                let messages_mutator = HavocScheduledMutator::with_max_stack_pow(
                    message_mutations(&generators_config),
                    6,
                );
                let mutator = LspInputMutator::new(text_document_mutator, messages_mutator);
                StdPowerMutationalStage::new(mutator)
            };
            let trigger_stop = common::trigger_stop_stage()?;
            let timeout_stop = TimeoutStopStage::new(Duration::from_hours(self.time_budget));
            tuple_list![
                calibration_stage,
                mutation_stage,
                stats_stage,
                timeout_stop,
                trigger_stop,
            ]
        };

        let asan_observer = asan_enabled.then_some(asan_observer);
        if asan_observer.is_some() {
            info!("Crash stack hashing will be enabled");
        }
        let mut executor = {
            const INPUT_SHM_SIZE: usize = 15 * 1024 * 1024 * 1024;
            let test_case_shmem = shmem_provider
                .new_shmem(INPUT_SHM_SIZE)
                .context("Creating shared memory for test case passing")?;
            let fuzz_input = FuzzInput::SharedMemory(test_case_shmem);
            let target_info = common::create_target_info(&self.execution, &binary_info);
            let workspace_observer = WorkspaceObserver::new(temp_dir);
            let exec_config = FuzzExecutionConfig {
                debug_child: self.execution.debug_child,
                debug_afl: self.execution.debug_afl,
                fuzz_input,
                auto_tokens: tokens.as_mut(),
                coverage_shm_info: (coverage_map_shmem_id, cov_observer.as_ref().len()),
                map_observer: cov_observer,
                asan_observer,
                other_observers: tuple_list![workspace_observer, time_observer],
            };
            LspExecutor::start(target_info, exec_config).context("Starting executor")?
        };

        common::process_tokens(&mut state, tokens);

        let mut event_manager = {
            let monitor = SimpleMonitor::new(|it| info!("{}", it));
            SimpleEventManager::new(monitor)
        };

        // In case the corpus is empty (on first run), reset
        if state.must_load_initial_inputs() {
            info!("Generating seeds");
            let mut generator = LspInputGenerator::new(&grammar_ctx);
            state
                .generate_initial_inputs_forced(
                    &mut fuzzer,
                    &mut executor,
                    &mut generator,
                    &mut event_manager,
                    self.generate_seeds,
                )
                .context("Generating initial input")?;
            info!(seeds = %state.corpus().count(), "Seed generation completed");
        }

        common::set_cpu_affinity(self.cpu_affinity);

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
