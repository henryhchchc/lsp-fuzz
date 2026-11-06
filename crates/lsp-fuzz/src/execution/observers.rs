use libafl::{
    executors::ExitKind,
    observers::{AsanBacktraceObserver, Observer, ObserversTuple},
};
use libafl_bolts::{
    Named,
    tuples::{MatchName, type_eq},
};
use serde::{Deserialize, Serialize};

use super::responses::LspOutputObserver;

#[derive(Debug, Serialize, Deserialize)]
pub struct Observers<MO, OBS> {
    pub(super) map_observer: MO,
    pub(super) asan_observer: Option<AsanBacktraceObserver>,
    pub(super) responses_observer: LspOutputObserver,
    pub(super) extra: OBS,
}

impl<MO, OBS> MatchName for Observers<MO, OBS>
where
    MO: Named,
    OBS: MatchName,
{
    fn match_name<T>(&self, name: &str) -> Option<&T> {
        if type_eq::<T, MO>() && self.map_observer.name() == name {
            Some(unsafe { &*(&raw const self.map_observer).cast::<T>() })
        } else if let Some(ref asan_observer) = self.asan_observer
            && type_eq::<T, AsanBacktraceObserver>()
            && asan_observer.name() == name
        {
            Some(unsafe { &*std::ptr::from_ref(asan_observer).cast::<T>() })
        } else if type_eq::<T, LspOutputObserver>() && self.responses_observer.name() == name {
            Some(unsafe { &*(&raw const self.responses_observer).cast::<T>() })
        } else {
            #[allow(deprecated, reason = "Fallback call")]
            self.extra.match_name(name)
        }
    }

    fn match_name_mut<T>(&mut self, name: &str) -> Option<&mut T> {
        if type_eq::<T, MO>() && self.map_observer.name() == name {
            Some(unsafe { &mut *(&raw mut self.map_observer).cast::<T>() })
        } else if let Some(ref mut asan_observer) = self.asan_observer
            && type_eq::<T, AsanBacktraceObserver>()
            && asan_observer.name() == name
        {
            Some(unsafe { &mut *std::ptr::from_mut(asan_observer).cast::<T>() })
        } else if type_eq::<T, LspOutputObserver>() && self.responses_observer.name() == name {
            Some(unsafe { &mut *(&raw mut self.responses_observer).cast::<T>() })
        } else {
            #[allow(deprecated, reason = "Fallback call")]
            self.extra.match_name_mut(name)
        }
    }
}

impl<I, State, MO, OBS> ObserversTuple<I, State> for Observers<MO, OBS>
where
    MO: Observer<I, State>,
    OBS: ObserversTuple<I, State>,
{
    fn pre_exec_all(&mut self, state: &mut State, input: &I) -> Result<(), libafl::Error> {
        self.map_observer.pre_exec(state, input)?;
        self.responses_observer.pre_exec(state, input)?;
        if let Some(ref mut asan_observer) = self.asan_observer {
            asan_observer.pre_exec(state, input)?;
        }
        self.extra.pre_exec_all(state, input)?;
        Ok(())
    }

    fn post_exec_all(
        &mut self,
        state: &mut State,
        input: &I,
        exit_kind: &ExitKind,
    ) -> Result<(), libafl::Error> {
        self.extra.post_exec_all(state, input, exit_kind)?;
        if let Some(ref mut asan_observer) = self.asan_observer {
            asan_observer.post_exec(state, input, exit_kind)?;
        }
        self.responses_observer.post_exec(state, input, exit_kind)?;
        self.map_observer.post_exec(state, input, exit_kind)?;
        Ok(())
    }
}
