use fst::raw::{Fst, Output, Node};
use fst::Streamer;
use fst::automaton::{Automaton, AlwaysMatch};


#[derive(Clone, Debug)]
struct StreamState<'f, S> {
    node: Node<'f>,
    trans: usize,
    out: Output,
    aut_state: S,
}

pub struct SimpleStateStream<'f, A=AlwaysMatch> where A: Automaton {
    fst: &'f Fst,
    aut: A,
    inp: Vec<u8>,
    stack: Vec<StreamState<'f, A::State>>,
}

impl<'f, A: Automaton> SimpleStateStream<'f, A> {
    fn new(fst: &'f Fst, aut: A) -> Self {
        let stack = vec![StreamState {
            node: fst.root(),
            trans: 0,
            out: Output::zero(),
            aut_state: aut.start(),
        }];
        SimpleStateStream {
            fst: fst,
            aut: aut,
            inp: Vec::with_capacity(16),
            stack: stack,
        }
    }
}

impl<'f, 'a, A: Automaton> Streamer<'a> for SimpleStateStream<'f, A>
        where A::State: 'a + Clone {
    type Item = (&'a [u8], Output, A::State);

    fn next(&'a mut self) -> Option<Self::Item> {
        while let Some(state) = self.stack.pop() {
            if state.trans >= state.node.len()
                    || !self.aut.can_match(&state.aut_state) {
                if state.node.addr() != self.fst.root().addr() {
                    self.inp.pop().unwrap();
                }
                continue;
            }
            let trans = state.node.transition(state.trans);
            let out = state.out.cat(trans.out);
            let next_state = self.aut.accept(&state.aut_state, trans.inp);
            let is_match = self.aut.is_match(&next_state);
            let next_node = self.fst.node(trans.addr);
            self.inp.push(trans.inp);
            self.stack.push(StreamState {
                trans: state.trans + 1, .. state
            });
            if next_node.is_final() && is_match {
                let cloned_next_state = next_state.clone();
                self.stack.push(StreamState {
                    node: next_node,
                    trans: 0,
                    out: out,
                    aut_state: next_state,
                });
                return Some((
                    &self.inp,
                    out.cat(next_node.final_output()),
                    cloned_next_state));
            } else {
                self.stack.push(StreamState {
                    node: next_node,
                    trans: 0,
                    out: out,
                    aut_state: next_state,
                });
            }
        }
        None
    }
}

pub trait FstExt {
    fn search_state_stream<A: Automaton>(&self, aut: A) -> SimpleStateStream<A>;
}

impl FstExt for Fst {
    fn search_state_stream<A: Automaton>(&self, aut: A) -> SimpleStateStream<A> {
        SimpleStateStream::new(self, aut)
    }

    /*fn search_openfst<A: Automaton>(&self, aut: A) -> Stream<A> {
        Stream::new(self, aut)
    }*/
}
