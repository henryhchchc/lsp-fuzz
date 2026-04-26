use std::{
    io::{self, Write},
    marker::PhantomData,
};

use libafl::{
    HasNamedMetadata,
    corpus::Corpus,
    feedbacks::{MapFeedback, MapFeedbackMetadata},
    observers::MapObserver,
    stages::{Restartable, Stage},
    state::{HasCorpus, HasExecutions, HasSolutions, HasStartTime},
};
use libafl_bolts::{Named, current_time, serdeany::SerdeAny};

#[derive(Debug)]
pub struct StatsStage<W, O, I> {
    stats_writer: W,
    coverage_feedback_name: String,
    _phantom: PhantomData<(O, I)>,
}

impl<W, O, I, State> Restartable<State> for StatsStage<W, O, I> {
    fn should_restart(&mut self, _state: &mut State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut State) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl<E, EM, State, Z, W, I, O> Stage<E, EM, State, Z> for StatsStage<W, O, I>
where
    W: Write,
    State: HasCorpus<I> + HasSolutions<I> + HasExecutions + HasStartTime + HasNamedMetadata,
    O: MapObserver,
    MapFeedbackMetadata<O::Entry>: SerdeAny,
{
    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut State,
        _manager: &mut EM,
    ) -> Result<(), libafl::Error> {
        let corpus_count = state.corpus().count();
        let solutions_count = state.solutions().count();
        let time = current_time()
            .checked_sub(*state.start_time())
            .unwrap_or_default()
            .as_secs();
        let exec = *state.executions();

        let cov_feedback_meta =
            state.named_metadata::<MapFeedbackMetadata<O::Entry>>(&self.coverage_feedback_name)?;
        let edges_found = cov_feedback_meta.num_covered_map_indexes;

        self.write_stat(corpus_count, solutions_count, time, exec, edges_found)
            .map_err(|err| libafl::Error::unknown(format!("Writing stat: {err}")))?;
        Ok(())
    }
}

impl<W, O, I> StatsStage<W, O, I> {
    pub fn new<C, N, R>(stats_writer: W, map_feedback: &MapFeedback<C, N, O, R>) -> Self {
        Self {
            stats_writer,
            coverage_feedback_name: map_feedback.name().clone().into_owned(),
            _phantom: PhantomData,
        }
    }

    fn write_stat(
        &mut self,
        corpus_count: usize,
        solutions_count: usize,
        time: u64,
        exec: u64,
        edges_found: usize,
    ) -> io::Result<()>
    where
        W: Write,
    {
        writeln!(
            self.stats_writer,
            "{corpus_count},{solutions_count},{time},{exec},{edges_found}"
        )?;
        self.stats_writer.flush()?;
        Ok(())
    }
}
