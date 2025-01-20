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

impl<F, S, E, EM, Z> Stage<E, EM, S, Z> for ActionStage<F>
where
    F: for<'a> Fn(&'a mut S),
{
    fn should_restart(&mut self, _state: &mut S) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut S) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut S,
        _manager: &mut EM,
    ) -> Result<(), libafl::Error> {
        (self.action)(state);
        Ok(())
    }
}
