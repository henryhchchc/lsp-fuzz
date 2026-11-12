use std::{marker::PhantomData, sync::mpsc::Receiver, time::Duration};

use libafl::{
    events::{Event, EventFirer, EventWithStats},
    stages::{Restartable, Stage},
    state::{HasExecutions, HasStartTime},
};

#[derive(Debug)]
pub struct StopOnReceived<I> {
    receiver: Receiver<()>,
    _input: PhantomData<I>,
}

impl<I> StopOnReceived<I> {
    /// Creates a stage that requests a stop when the receiver yields a message.
    #[must_use]
    pub const fn new(receiver: Receiver<()>) -> Self {
        Self {
            receiver,
            _input: PhantomData,
        }
    }
}

impl<I, State> Restartable<State> for StopOnReceived<I> {
    fn should_restart(&mut self, _state: &mut State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, M, Z, I, State> Stage<E, M, State, Z> for StopOnReceived<I>
where
    State: HasExecutions,
    M: EventFirer<I, State>,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        manager: &mut M,
    ) -> Result<(), libafl::Error> {
        if self.receiver.try_recv().is_ok() {
            let executions = state.executions();
            let event = EventWithStats::with_current_time(Event::Stop, *executions);
            manager.fire(state, event)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TimeoutStopStage<I> {
    timeout: Duration,
    _input: PhantomData<I>,
}

impl<I> TimeoutStopStage<I> {
    /// Creates a stage that requests a stop after the configured runtime.
    #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            _input: PhantomData,
        }
    }
}

impl<I, State> Restartable<State> for TimeoutStopStage<I> {
    fn should_restart(&mut self, _state: &mut State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, M, Z, I, State> Stage<E, M, State, Z> for TimeoutStopStage<I>
where
    State: HasStartTime + HasExecutions,
    M: EventFirer<I, State>,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        manager: &mut M,
    ) -> Result<(), libafl::Error> {
        let start_time = *state.start_time();
        let current_time = libafl_bolts::current_time();
        if current_time.checked_sub(start_time).unwrap_or_default() > self.timeout {
            let executions = state.executions();
            let event = EventWithStats::with_current_time(Event::Stop, *executions);
            manager.fire(state, event)?;
        }
        Ok(())
    }
}
