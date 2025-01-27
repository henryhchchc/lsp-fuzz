use std::{collections::HashMap, env::temp_dir, ops::Not, path::PathBuf, time::Duration};

use anyhow::{bail, Context};
use clap::builder::BoolishValueParser;
use core_affinity::CoreId;
use libafl::{
    corpus::{InMemoryCorpus, NopCorpus},
    events::SimpleEventManager,
    feedbacks::CrashFeedback,
    monitors::SimpleMonitor,
    mutators::{StdScheduledMutator, Tokens},
    observers::{AsanBacktraceObserver, StdMapObserver},
    schedulers::QueueScheduler,
    stages::StdTMinMutationalStage,
    state::StdState,
    Fuzzer, HasMetadata, StdFuzzer,
};
use libafl_bolts::shmem::ShMem;
use libafl_bolts::{
    current_nanos,
    fs::InputFile,
    rands::StdRand,
    shmem::{ShMemProvider, UnixShMemProvider},
    tuples::Handled,
    AsSliceMut,
};
use lsp_fuzz::{
    execution::{FuzzExecutionConfig, FuzzInput, FuzzTargetInfo, LspExecutor},
    fuzz_target::{StaticTargetBinaryInfo, TargetBinaryInfo},
    lsp_input::{messages::message_reductions, LspInputMutator},
    stages::CleanupWorkspaceDirs,
    text_document::{text_document_reductions, Language},
};
use tracing::{error, info, warn};
use tuple_list::tuple_list;

use crate::{
    cli::fuzz::replace_at_args, fuzzing::ExecutorOptions, language_fragments::load_grammar_lookup,
};

use super::{parse_hash_map, GlobalOptions};

/// Fuzz a Language Server Protocol (LSP) server.
#[derive(Debug, clap::Parser)]
pub(super) struct MinimizeCommand {
    /// Directory containing seed inputs to minimize.
    #[clap(long)]
    input: PathBuf,

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

    #[clap(long)]
    cpu_affinity: Option<usize>,

    #[clap(long, value_parser = parse_hash_map::<Language, PathBuf>)]
    language_fragments: HashMap<Language, PathBuf>,
}

impl MinimizeCommand {
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
            self.execution.debug_afl,
            self.execution.debug_child,
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

        let corpus = InMemoryCorpus::new();

        let random_seed = global_options.random_seed.unwrap_or_else(current_nanos);
        let mut state = StdState::new(
            StdRand::with_seed(random_seed),
            corpus,
            NopCorpus::new(),
            &mut (),
            &mut (),
        )
        .context("Creating state")?;

        let mut tokens = self.no_auto_dict.not().then(Tokens::new);

        let scheduler = QueueScheduler::new();

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, (), ());

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

        info!("Loading grammar context");
        let grammar_ctx =
            load_grammar_lookup(&self.language_fragments).context("Creating grammar context")?;

        let mut fuzz_stages = {
            let doc_reducer =
                StdScheduledMutator::with_max_stack_pow(text_document_reductions(&grammar_ctx), 2);
            let req_reducer = StdScheduledMutator::with_max_stack_pow(message_reductions(), 2);
            let mutator = LspInputMutator::new(doc_reducer, req_reducer);
            let tmin = StdTMinMutationalStage::new(mutator, CrashFeedback::new(), 128);
            let cleanup_workspace_stage = CleanupWorkspaceDirs::new(temp_dir_str.to_owned(), 1000);
            tuple_list![tmin, cleanup_workspace_stage]
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

        let coverage_map_info = Some((cov_map_shmem_id, cov_map_observer.as_ref().len()));

        let target_info = FuzzTargetInfo {
            path: self.execution.lsp_executable,
            args: self.execution.target_args,
            persistent_fuzzing: binary_info.is_persistent_mode,
            defer_fork_server: binary_info.is_defer_fork_server,
            crash_exit_code: self.execution.crash_exit_code,
            timeout: Duration::from_millis(self.execution.timeout).into(),
            kill_signal: self.execution.kill_signal,
            env: self.execution.target_env,
        };

        let exec_config = FuzzExecutionConfig {
            debug_child: self.execution.debug_child,
            debug_afl: self.execution.debug_afl,
            fuzz_input,
            auto_tokens: tokens.as_mut(),
            coverage_map_info,
            map_observer: cov_map_observer,
            asan_observer_handle: asan_handle,
            other_observers: tuple_list![asan_observer],
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

        if let Some(id) = self.cpu_affinity {
            let core_id = CoreId { id };
            if core_affinity::set_for_current(core_id) {
                info!("Set CPU affinity to core {id}");
            } else {
                warn!("Failed to set CPU affinity to core {id}");
            }
        }

        state
            .load_initial_inputs_forced(
                &mut fuzzer,
                &mut executor,
                &mut event_manager,
                &[self.input],
            )
            .context("Loading input")?;

        fuzzer
            .fuzz_loop(
                &mut fuzz_stages,
                &mut executor,
                &mut state,
                &mut event_manager,
            )
            .context("In fuzz loop")
    }
}
