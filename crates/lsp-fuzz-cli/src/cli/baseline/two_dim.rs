use std::{fs::OpenOptions, io::BufWriter, ops::Not, path::PathBuf, time::Duration};

use anyhow::Context;
use clap::builder::BoolishValueParser;
use libafl::{
    Fuzzer, NopInputFilter, StdFuzzerBuilder,
    events::SimpleEventManager,
    feedback_or,
    feedbacks::{MaxMapFeedback, TimeFeedback},
    generators::{NautilusContext, NautilusGenerator},
    inputs::NautilusBytesConverter,
    monitors::SimpleMonitor,
    mutators::{
        HavocScheduledMutator, NautilusRandomMutator, NautilusRecursionMutator,
        NautilusSpliceMutator, havoc_mutations_no_crossover,
    },
    observers::{
        AsanBacktraceObserver, CanTrack, HitcountsMapObserver, StdMapObserver, TimeObserver,
    },
    schedulers::powersched::BaseSchedule,
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::StdState,
};
use libafl_bolts::{
    AsSliceMut, HasLen,
    rands::StdRand,
    shmem::{ShMem, ShMemProvider, StdShMemProvider},
};
use lsp_fuzz::{
    baseline::{
        BaselineByteConverter, BaselineMessageMutator, BaselineSequenceMutator,
        two_dim::{TwoDimBaselineMutator, TwoDimInputConverter},
    },
    corpus::{TestCaseFileNameFeedback, corpus_kind::CORPUS},
    execution::{
        FuzzExecutionConfig, FuzzInput, LspExecutor, responses::LspOutputObserver,
        workspace_observer::WorkspaceObserver,
    },
    fuzz_target,
    stages::{StatsStage, StopOnReceived, TimeoutStopStage},
    utf8::UTF8Tokens,
};
use lsp_fuzz_grammars::Language;
use tracing::info;
use tuple_list::tuple_list;

use crate::{
    cli::GlobalOptions,
    fuzzing::{ExecutorOptions, FuzzerStateDir, common},
};

/// Fuzz a Language Server Protocol (LSP) server using BytesInput.
#[derive(Debug, clap::Parser)]
pub struct TwoDimyBaseline {
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

    #[clap(long)]
    seeds: PathBuf,

    /// Maximum size of generated inputs in bytes.
    #[clap(long, default_value_t = 8192)]
    max_input_size: usize,

    #[clap(long)]
    language: Language,
}

impl TwoDimyBaseline {
    pub fn run(self, global_options: GlobalOptions) -> Result<(), anyhow::Error> {
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

        info!("Setting up fuzzing environment");

        let coverage_map_observer = {
            let shmem_buf = coverage_shmem.as_slice_mut();
            // SAFETY: We never move the piece of the shared memory.
            unsafe { StdMapObserver::new("edges", shmem_buf) }
        };

        let asan_observer = AsanBacktraceObserver::new("asan_stacktrace");

        let asan_enabled = binary_info.uses_address_sanitizer && self.no_asan.not();
        let cov_observer = HitcountsMapObserver::new(coverage_map_observer).track_indices();

        let temp_dir = self.temp_dir.unwrap_or_else(std::env::temp_dir);

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        let map_feedback = MaxMapFeedback::new(&cov_observer);
        let calibration_stage = CalibrationStage::new(&map_feedback);
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
            TestCaseFileNameFeedback::<CORPUS>::new(),
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

        let mut nautilus_ctx = NautilusContext {
            ctx: lsp_fuzz::lsp::metamodel::get_nautilus_context(),
        };
        nautilus_ctx.ctx.initialize(65535);

        let scheduler = common::scheduler(
            &mut state,
            &cov_observer,
            self.power_schedule,
            self.cycle_power_schedule,
        );

        let target_bytes_converter = TwoDimInputConverter::new(
            temp_dir.clone(),
            self.language.lsp_language_id().to_owned(),
            BaselineByteConverter::new(NautilusBytesConverter::new(&nautilus_ctx)),
        );

        // A fuzzer with feedback and a corpus scheduler
        let mut fuzzer = StdFuzzerBuilder::new()
            .bytes_converter(target_bytes_converter)
            .input_filter(NopInputFilter)
            .build(scheduler, feedback, objective)
            .context("Building fuzzer")?;

        let mut fuzz_stages = {
            // Create a standard havoc mutator for BytesInput
            let havoc_mutations = havoc_mutations_no_crossover();
            let nautllus_mutator = tuple_list![
                BaselineMessageMutator::new(NautilusRandomMutator::new(&nautilus_ctx)),
                BaselineMessageMutator::new(NautilusRandomMutator::new(&nautilus_ctx)),
                BaselineMessageMutator::new(NautilusRandomMutator::new(&nautilus_ctx)),
                BaselineMessageMutator::new(NautilusSpliceMutator::new(&nautilus_ctx)),
                BaselineMessageMutator::new(NautilusSpliceMutator::new(&nautilus_ctx)),
                BaselineMessageMutator::new(NautilusSpliceMutator::new(&nautilus_ctx)),
                BaselineMessageMutator::new(NautilusRecursionMutator::new(&nautilus_ctx)),
                BaselineSequenceMutator::new(NautilusGenerator::new(&nautilus_ctx)),
            ];

            let mutator = TwoDimBaselineMutator::new(
                HavocScheduledMutator::new(havoc_mutations),
                HavocScheduledMutator::new(nautllus_mutator),
            );

            let mutation_stage = StdPowerMutationalStage::new(mutator);
            let trigger_stop = trigger_stop_stage()?;
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
                responses_observer: LspOutputObserver::new(),
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

        common::set_cpu_affinity(self.cpu_affinity);

        let mut nautilus_ctx = libafl::generators::NautilusContext {
            ctx: lsp_fuzz::lsp::metamodel::get_nautilus_context(),
        };
        nautilus_ctx.ctx.initialize(65535);

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
    common::trigger_stop_stage()
}
