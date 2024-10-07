use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use libafl::{
    corpus::{Corpus, InMemoryCorpus, OnDiskCorpus},
    events::SimpleEventManager,
    feedback_and_fast, feedback_or,
    inputs::BytesInput,
    prelude::{
        havoc_mutations, powersched::PowerSchedule, tokens_mutations, CanTrack, CrashFeedback,
        ForkserverExecutor, Generator, HasObservers, HitcountsMapObserver, MaxMapFeedback,
        PowerQueueScheduler, RandBytesGenerator, SimpleMonitor, StdMapObserver,
        StdScheduledMutator, TimeFeedback, TimeObserver, Tokens,
    },
    stages::{CalibrationStage, StdPowerMutationalStage},
    state::{HasCorpus, StdState},
    Evaluator, Fuzzer, HasMetadata, StdFuzzer,
};
use libafl_bolts::{
    current_nanos,
    rands::StdRand,
    shmem::{ShMem, ShMemProvider, UnixShMemProvider},
    tuples::{tuple_list, Handled, Merge},
    AsSliceMut, Truncate,
};
use nix::sys::signal::Signal;
use tracing::{info, warn};

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

    #[clap(long, short, default_value = "SIGKILL")]
    kill_signal: Signal,

    #[clap(long, default_value_t = false)]
    debug_child: bool,
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
            .context("Writing shared memory address to env")?;

        // Create an observation channel using the signals map
        let shmem_observer = {
            let shmem_buf = shmem.as_slice_mut();
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
        let combined_feedback = feedback_and_fast!(
            // Must be a crash
            CrashFeedback::new(),
            // Take it only if trigger new coverage over crashes
            // Uses `with_name` to create a different history from the `MaxMapFeedback` in `feedback` above
            MaxMapFeedback::with_name("mapfeedback_metadata_objective", &edges_observer)
        );
        let mut objective = combined_feedback;

        let corpus = InMemoryCorpus::<BytesInput>::new();
        let solution_corpus =
            OnDiskCorpus::new(self.crashes).context("Creating solution corpus")?;

        // create a State from scratch
        let mut state = StdState::new(
            // RNG
            StdRand::with_seed(current_nanos()),
            corpus,
            solution_corpus,
            // States of the feedbacks.
            // The feedbacks can report the data that should persist in the State.
            &mut feedback,
            // Same for objective feedbacks
            &mut objective,
        )
        .unwrap();

        // The Monitor trait define how the fuzzer stats are reported to the user
        let monitor = SimpleMonitor::new(|s| info!("{s}"));

        // The event manager handle the various events generated during the fuzzing loop
        // such as the notification of the addition of a new item to the corpus
        let mut mgr = SimpleEventManager::new(monitor);

        let observer_ref = edges_observer.handle();

        let mut tokens = Tokens::new();

        // Setup a mutational stage with a basic bytes mutator
        let mutator =
            StdScheduledMutator::with_max_stack_pow(havoc_mutations().merge(tokens_mutations()), 6);

        let scheduler = PowerQueueScheduler::new(&mut state, &edges_observer, PowerSchedule::FAST);

        let power_mutation_stage = StdPowerMutationalStage::new(mutator);

        let mut stages = tuple_list!(calibration_stage, power_mutation_stage);

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        let mut executor = ForkserverExecutor::builder()
            .program(self.lsp_executable)
            .debug_child(self.debug_child)
            .shmem_provider(&mut shmem_provider)
            .autotokens(&mut tokens)
            .parse_afl_cmdline(self.child_args)
            .coverage_map_size(self.shared_memory_size)
            .timeout(Duration::from_millis(self.timeout))
            .kill_signal(self.kill_signal)
            .build(tuple_list!(time_observer, edges_observer))
            .context("Creating executor")?;

        if let Some(dynamic_map_size) = executor.coverage_map_size() {
            executor.observers_mut()[&observer_ref]
                .as_mut()
                .truncate(dynamic_map_size);
            info!("Resizing coverage map to {}", dynamic_map_size);
        }

        // In case the corpus is empty (on first run), reset
        if state.must_load_initial_inputs() {
            if let Some(seeds_dir) = self.seeds_dir {
                state
                    .load_initial_inputs(&mut fuzzer, &mut executor, &mut mgr, &[seeds_dir])
                    .context("Loading seed inputs")?;
                info!(num_inputs = state.corpus().count(), "Seed inputs imported");
            } else {
                warn!("No seed inputs provided, starting from scratch");
                let mut generator = RandBytesGenerator::new(256);
                let initial_input = generator
                    .generate(&mut state)
                    .context("Generating initial input")?;
                fuzzer
                    .add_input(&mut state, &mut executor, &mut mgr, initial_input)
                    .context("Adding initial input")?;
            }
        }

        state.add_metadata(tokens);

        fuzzer
            .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
            .context("In fuzzloop")
    }
}
