use std::{hash::Hash, iter, path::Path, sync::mpsc, time::Duration};

use anyhow::Context;
use core_affinity::CoreId;
use libafl::{
    HasMetadata, HasNamedMetadata,
    corpus::{CachedOnDiskCorpus, HasTestcase, InMemoryOnDiskCorpus, OnDiskCorpus},
    feedbacks::{
        ConstFeedback, CrashFeedback, FastAndFeedback, FastOrFeedback, Feedback, NewHashFeedback,
    },
    inputs::Input,
    observers::{AsanBacktraceObserver, CanTrack},
    schedulers::{
        IndexesLenTimeMinimizerScheduler, Scheduler, StdWeightedScheduler,
        powersched::{BaseSchedule, PowerSchedule},
    },
    state::{HasCorpus, HasRand},
};
use libafl_bolts::{HasLen, Named, tuples::MatchName};
use lsp_fuzz::{
    execution::FuzzTargetInfo, fuzz_target::StaticTargetBinaryInfo, stages::StopOnReceived,
    utf8::UTF8Tokens,
};
use rayon::prelude::*;
use tracing::{info, warn};

use crate::fuzzing::ExecutorOptions;

pub fn scheduler<State, I, C, O>(
    state: &mut State,
    cov_observer: &C,
    power_schedule: BaseSchedule,
    cycle_power_schedule: bool,
) -> impl Scheduler<I, State> + use<State, I, C, O>
where
    C: Named + CanTrack + AsRef<O>,
    I: HasLen,
    State: HasMetadata + HasCorpus<I> + HasRand + HasTestcase<I>,
    O: Hash,
{
    let power_schedule = PowerSchedule::new(power_schedule);
    let mut weighted_scheduler =
        StdWeightedScheduler::with_schedule(state, cov_observer, Some(power_schedule));
    if cycle_power_schedule {
        weighted_scheduler = weighted_scheduler.cycling_scheduler();
    }
    IndexesLenTimeMinimizerScheduler::new(cov_observer, weighted_scheduler)
}

pub fn objective<EM, I, Observers, State>(
    asan_enabled: bool,
    asan_observer: &AsanBacktraceObserver,
) -> impl Feedback<EM, I, Observers, State> + use<EM, I, Observers, State>
where
    Observers: MatchName,
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
) -> anyhow::Result<(InMemoryOnDiskCorpus<I>, OnDiskCorpus<I>)>
where
    I: Input,
{
    let corpus = InMemoryOnDiskCorpus::with_meta_format_and_prefix(corpus_path, None, None, false)
        .context("Creating corpus")?;

    let solutions = OnDiskCorpus::with_meta_format_and_prefix(solution_path, None, None, false)
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

pub trait ParTryCollect<T, E>: ParallelIterator {
    fn try_collect_par<C>(self) -> Result<C, E>
    where
        C: Default + IntoIterator<Item = T> + FromIterator<T> + Send,
        E: Send;
}

impl<Iter, T, E> ParTryCollect<T, E> for Iter
where
    Iter: ParallelIterator<Item = Result<T, E>>,
{
    fn try_collect_par<C>(self) -> Result<C, E>
    where
        C: Default + IntoIterator<Item = T> + FromIterator<T> + Send,
        E: Send,
    {
        self.try_fold(C::default, |acc, item| {
            Ok(acc.into_iter().chain(iter::once(item?)).collect())
        })
        .try_reduce_with(|lhs, rhs| Ok(lhs.into_iter().chain(rhs).collect()))
        .unwrap_or_else(|| Ok(C::default()))
    }
}
