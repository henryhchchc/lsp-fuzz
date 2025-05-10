use std::{path::Path, sync::mpsc, time::Duration};

use anyhow::Context;
use core_affinity::CoreId;
use libafl::{
    HasMetadata, HasNamedMetadata,
    corpus::{CachedOnDiskCorpus, OnDiskCorpus, ondisk::OnDiskMetadataFormat},
    feedbacks::{
        ConstFeedback, CrashFeedback, FastAndFeedback, FastOrFeedback, Feedback, NewHashFeedback,
    },
    inputs::Input,
    observers::AsanBacktraceObserver,
};
use libafl_bolts::tuples::MatchName;
use lsp_fuzz::{
    execution::FuzzTargetInfo, fuzz_target::StaticTargetBinaryInfo, stages::StopOnReceived,
    utf8::UTF8Tokens,
};
use tracing::{info, warn};

use crate::fuzzing::ExecutorOptions;

pub fn objective<EM, I, OT, State>(
    asan_enabled: bool,
    asan_observer: &AsanBacktraceObserver,
) -> impl Feedback<EM, I, OT, State> + use<EM, I, OT, State>
where
    OT: MatchName,
    State: HasNamedMetadata,
{
    FastAndFeedback::new(
        CrashFeedback::new(),
        FastOrFeedback::new(
            ConstFeedback::new(!asan_enabled),
            NewHashFeedback::new(asan_observer),
        ),
    )
}

pub fn create_corpus<I>(
    corpus_path: &Path,
    solution_path: &Path,
) -> anyhow::Result<(CachedOnDiskCorpus<I>, OnDiskCorpus<I>)>
where
    I: Input,
{
    const CACHE_LEN: usize = 16384;
    let corpus = CachedOnDiskCorpus::with_meta_format(
        corpus_path,
        CACHE_LEN,
        Some(OnDiskMetadataFormat::Json),
    )
    .context("Creating corpus")?;

    let solutions = OnDiskCorpus::with_meta_format(solution_path, OnDiskMetadataFormat::Json)
        .context("Creating solution corpus")?;

    Ok((corpus, solutions))
}

/// Creates a target info struct from execution options and binary info.
pub fn create_target_info(
    options: &ExecutorOptions,
    binary_info: &StaticTargetBinaryInfo,
) -> FuzzTargetInfo {
    FuzzTargetInfo {
        path: options.lsp_executable.clone(),
        args: options.target_args.clone(),
        persistent_fuzzing: binary_info.is_persistent_mode,
        defer_fork_server: binary_info.is_defer_fork_server,
        crash_exit_code: options.crash_exit_code,
        timeout: Duration::from_millis(options.exec_timeout).into(),
        kill_signal: options.kill_signal,
        env: options.target_env.clone(),
    }
}

/// Sets CPU affinity if requested.
pub fn set_cpu_affinity(core_id: Option<usize>) {
    if let Some(id) = core_id {
        let core_id = CoreId { id };
        if core_affinity::set_for_current(core_id) {
            info!("Set CPU affinity to core {id}");
        } else {
            warn!("Failed to set CPU affinity to core {id}");
        }
    }
}

/// Creates a stop stage that triggers when Ctrl+C is pressed.
pub fn trigger_stop_stage<I>() -> Result<StopOnReceived<I>, anyhow::Error> {
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

/// Process tokens extracted during fuzzing.
pub fn process_tokens<S>(state: &mut S, tokens: Option<UTF8Tokens>)
where
    S: HasMetadata,
{
    if let Some(tokens) = tokens {
        info!("Extracted {} UTF-8 token(s) from the target.", tokens.len());
        state.add_metadata(tokens);
    }
}

/// Analyzes the fuzz target and returns information about its instrumentation status.
pub fn analyze_fuzz_target(target_path: &Path) -> Result<StaticTargetBinaryInfo, anyhow::Error> {
    info!("Analyzing fuzz target");
    let binary_info = StaticTargetBinaryInfo::scan(target_path).context("Analyzing fuzz target")?;

    if !binary_info.is_afl_instrumented {
        anyhow::bail!("The fuzz target is not instrumented with AFL++");
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

    Ok(binary_info)
}
