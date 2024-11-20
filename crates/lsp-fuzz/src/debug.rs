use std::{borrow::Cow, marker::PhantomData};

use derive_new::new as New;
use libafl::{
    stages::Stage,
    state::{State, UsesState},
};
use libafl_bolts::Named;

#[derive(Debug, New)]
pub struct ActionStage<F, S> {
    action: F,
    _state: PhantomData<S>,
}

impl<F, S> Named for ActionStage<F, S> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ActionStage");
        &NAME
    }
}

impl<F, S> UsesState for ActionStage<F, S>
where
    S: State,
{
    type State = S;
}

impl<F, S, E, EM, Z> Stage<E, EM, Z> for ActionStage<F, S>
where
    F: for<'a> Fn(&'a mut S),
    S: State,
    E: UsesState<State = Self::State>,
    EM: UsesState<State = Self::State>,
    Z: UsesState<State = Self::State>,
{
    fn should_restart(&mut self, _state: &mut Self::State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut Self::State) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut Self::State,
        _manager: &mut EM,
    ) -> Result<(), libafl::Error> {
        (self.action)(state);
        Ok(())
    }
}
