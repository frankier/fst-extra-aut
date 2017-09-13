use fst::map::Map;
use ext::raw;
use ext::raw::FstExt;
use fst::automaton::{Automaton, AlwaysMatch};
use fst::Streamer;

//pub struct Map(raw::Fst);

pub trait MapExt {
    fn search_state_stream<A: Automaton>(&self, aut: A) -> SimpleStateStream<A>;
}

impl MapExt for Map {
    fn search_state_stream<A: Automaton>(&self, aut: A) -> SimpleStateStream<A> {
        SimpleStateStream(self.0.search_state_stream(aut))
    }
}

pub struct SimpleStateStream<'m, A=AlwaysMatch>(raw::SimpleStateStream<'m, A>) where A: Automaton;

impl<'a, 'm, A: Automaton> Streamer<'a> for SimpleStateStream<'m, A>
        where A::State: 'a + Clone {
    type Item = (&'a [u8], u64, A::State);

    fn next(&'a mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, out, state)| (key, out.value(), state))
    }
}
