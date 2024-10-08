use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use libafl::{
    corpus::{Corpus, InMemoryCorpus, OnDiskCorpus},
    events::SimpleEventManager,
    feedback_and_fast, feedback_or,
    prelude::{
        havoc_mutations, powersched::PowerSchedule, tokens_mutations, CanTrack, CrashFeedback,
        Generator, HitcountsMapObserver, IndexesLenTimeMinimizerScheduler, MaxMapFeedback,
        PowerQueueScheduler, SimpleMonitor, StdMapObserver, StdScheduledMutator, TimeFeedback,
        TimeObserver, Tokens,
    },
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::{HasCorpus, StdState},
    Evaluator, Fuzzer, HasMetadata, StdFuzzer,
};
use libafl_bolts::{
    current_nanos,
    rands::StdRand,
    shmem::{ShMem, ShMemProvider, UnixShMemProvider},
    tuples::Merge,
    AsSliceMut,
};
use lsp_fuzz::{
    execution::LspExecutor, generator::LspInpuGenerator, inputs::LspInput, muation::LspInputMutator,
};
use nix::sys::signal::Signal;
use tracing::{info, warn};
use tuple_list::tuple_list;

use super::GlobalOptions;

#[derive(Debug, clap::Parser)]
pub(super) struct Cli {
    #[clap(long)]
    seeds_dir: Option<PathBuf>,

    #[clap(long, default_value = "crashes")]
    crashes: PathBuf,

    #[clap(long)]
    lsp_work_dir: Option<PathBuf>,

    #[clap(long)]
    lsp_executable: PathBuf,

    #[clap(long, short, default_value = "")]
    child_args: Vec<String>,

    #[clap(long, short, default_value_t = 65536)]
    shared_memory_size: usize,

    #[clap(long, short, default_value_t = 1200)]
    timeout: u64,

    #[clap(long, short, default_value_t = Signal::SIGKILL)]
    kill_signal: Signal,

    #[clap(long, default_value_t = false)]
    debug_child: bool,

    #[clap(long, value_enum, default_value_t = PowerSchedule::FAST)]
    power_schedule: PowerSchedule,
}

impl Cli {
    pub(super) fn run(self, global_options: GlobalOptions) -> Result<(), anyhow::Error> {
        let mut shmem_provider =
            UnixShMemProvider::new().context("Creating shared memory provider")?;

        // The coverage map shared between observer and executor
        let mut shmem = shmem_provider
            .new_shmem(self.shared_memory_size)
            .context("Creating shared memory")?;
        // let the forkserver know the shmid
        shmem
            .write_to_env("__AFL_SHM_ID")
            .context("Writing shared memory config to env")?;
        std::env::set_var("AFL_MAP_SIZE", format!("{}", self.shared_memory_size));

        // Create an observation channel using the signals map
        let shmem_observer = {
            let shmem_buf = shmem.as_slice_mut();
            // SAFETY: We never move the pirce of the shared memory.
            unsafe { StdMapObserver::new("shared_mem", shmem_buf) }
        };
        let edges_observer = HitcountsMapObserver::new(shmem_observer).track_indices();

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        // Feedback to rate the interestingness of an input
        // This one is composed by two Feedbacks in OR
        // New maximization map feedback linked to the edges observer and the feedback state
        let map_feedback = MaxMapFeedback::new(&edges_observer);
        let calibration_stage = CalibrationStage::new(&map_feedback);
        let mut feedback = feedback_or!(
            map_feedback,
            // Time feedback, this one does not need a feedback state
            TimeFeedback::new(&time_observer)
        );

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

        let scheduler = IndexesLenTimeMinimizerScheduler::new(
            &edges_observer,
            PowerQueueScheduler::new(&mut state, &edges_observer, self.power_schedule),
        );

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        let mut executor = LspExecutor::new(
            &self.lsp_executable,
            self.child_args,
            None,
            Duration::from_millis(self.timeout).into(),
            self.debug_child,
            self.kill_signal,
            Some(&mut tokens),
            edges_observer,
            tuple_list!(time_observer),
        )
        .context("Creating executor")?;

        let monitor = SimpleMonitor::new(|s| info!("{s}"));
        let mut mgr = SimpleEventManager::new(monitor);

        // In case the corpus is empty (on first run), reset
        if state.must_load_initial_inputs() {
            if let Some(seeds_dir) = self.seeds_dir {
                state
                    .load_initial_inputs(&mut fuzzer, &mut executor, &mut mgr, &[seeds_dir])
                    .context("Loading seed inputs")?;
                info!(num_inputs = state.corpus().count(), "Seed inputs imported");
            } else {
                warn!("No seed inputs provided, starting from scratch");
                let mut generator = LspInpuGenerator;
                let initial_input = generator
                    .generate(&mut state)
                    .context("Generating initial input")?;
                fuzzer
                    .add_input(&mut state, &mut executor, &mut mgr, initial_input)
                    .context("Adding initial input")?;
            }
        }

        state.add_metadata(tokens);

        let mutator = LspInputMutator::new(StdScheduledMutator::new(
            havoc_mutations().merge(tokens_mutations()),
        ));
        let power_mutation_stage = StdPowerMutationalStage::new(mutator);
        let mut stages = tuple_list!(calibration_stage, power_mutation_stage);

        fuzzer
            .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
            .context("In fuzzloop")
    }
}
