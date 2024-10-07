use std::{env::temp_dir, marker::PhantomData, path::Path};

use libafl::{
    inputs::UsesInput,
    prelude::{Executor, ExitKind, Forkserver},
    state::{HasExecutions, State, UsesState},
};
use libafl_bolts::{current_nanos, fs::InputFile};
use nix::{
    sys::{
        signal::{kill, Signal},
        time::TimeSpec,
    },
    unistd::Pid,
};

use crate::LspInput;

#[derive(Debug)]
pub struct LspExecutor<S> {
    fork_server: Forkserver,
    crash_exit_code: Option<i8>,
    kill_signal: Signal,
    timeout: TimeSpec,
    input_file: InputFile,
    _state: PhantomData<S>,
}

impl<S> LspExecutor<S> {
    pub fn new(
        fuzz_target: &Path,
        crash_exit_code: Option<i8>,
        timeout: TimeSpec,
        debug_child: bool,
        kill_signal: Signal,
    ) -> Result<Self, libafl::Error> {
        let filename = format!("lsp-fuzz-input_{}", current_nanos());
        let input_file_path = temp_dir().join(filename);
        let input_file = InputFile::create(input_file_path)?;
        let fork_server = Forkserver::with_kill_signal(
            fuzz_target.as_os_str().to_owned(),
            Vec::default(),
            Vec::default(),
            input_file.as_raw_fd(),
            true,
            0,
            false,
            false,
            debug_child,
            kill_signal,
        )?;
        Ok(Self {
            fork_server,
            crash_exit_code,
            kill_signal,
            timeout,
            input_file,
            _state: PhantomData,
        })
    }
}

impl<S> UsesState for LspExecutor<S>
where
    S: State + UsesInput<Input = LspInput>,
{
    type State = S;
}

impl<EM, Z, S> Executor<EM, Z> for LspExecutor<S>
where
    S: State + UsesInput<Input = LspInput> + HasExecutions,
    EM: UsesState<State = S>,
    Z: UsesState<State = S>,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        state: &mut Self::State,
        _mgr: &mut EM,
        input: &Self::Input,
    ) -> Result<ExitKind, libafl::Error> {
        *state.executions_mut() += 1;
        let mut exit_kind = ExitKind::Ok;
        let last_run_timed_out = self.fork_server.last_run_timed_out_raw();
        let input_bytes = input.bytes.clone();
        let input_size = input_bytes.as_slice().len();
        self.input_file
            .write_buf(&input_bytes.as_slice()[..input_size])?;
        let send_len = self.fork_server.write_ctl(last_run_timed_out)?;
        self.fork_server.set_last_run_timed_out(false);
        if send_len != 4 {
            return Err(libafl::Error::unknown(
                "Unable to request new process from fork server (OOM?)".to_string(),
            ));
        }
        let (recv_pid_len, pid) = self.fork_server.read_st()?;
        if recv_pid_len != 4 {
            return Err(libafl::Error::unknown(
                "Unable to request new process from fork server (OOM?)".to_string(),
            ));
        }
        if pid <= 0 {
            return Err(libafl::Error::unknown(
                "Fork server is misbehaving (OOM?)".to_string(),
            ));
        }
        self.fork_server.set_child_pid(Pid::from_raw(pid));
        if let Some(status) = self.fork_server.read_st_timed(&self.timeout)? {
            self.fork_server.set_status(status);
            let exitcode_is_crash = if let Some(crash_exitcode) = self.crash_exit_code {
                (libc::WEXITSTATUS(self.fork_server.status()) as i8) == crash_exitcode
            } else {
                false
            };
            if libc::WIFSIGNALED(self.fork_server.status()) || exitcode_is_crash {
                exit_kind = ExitKind::Crash;
            }
        } else {
            self.fork_server.set_last_run_timed_out(true);
            // We need to kill the child in case he has timed out, or we can't get the correct
            // pid in the next call to self.executor.forkserver_mut().read_st()?
            let _ = kill(self.fork_server.child_pid(), self.kill_signal);
            let (recv_status_len, _) = self.fork_server.read_st()?;
            if recv_status_len != 4 {
                return Err(libafl::Error::unknown(
                    "Could not kill timed-out child".to_string(),
                ));
            }
            exit_kind = ExitKind::Timeout;
        }
        if !libc::WIFSTOPPED(self.fork_server.status()) {
            self.fork_server.reset_child_pid();
        }
        Ok(exit_kind)
    }
}
