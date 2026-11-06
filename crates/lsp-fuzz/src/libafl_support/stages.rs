use std::borrow::Cow;

use derive_new::new as New;
use libafl::stages::Stage;
use libafl_bolts::Named;

#[derive(Debug, New)]
pub struct ActionStage<F> {
    action: F,
}

impl<F> Named for ActionStage<F> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ActionStage");
        &NAME
    }
}

impl<F, State, E, EM, Z> Stage<E, EM, State, Z> for ActionStage<F>
where
    F: for<'a> Fn(&'a mut State),
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        _manager: &mut EM,
    ) -> Result<(), libafl::Error> {
        (self.action)(state);
        Ok(())
    }
}
