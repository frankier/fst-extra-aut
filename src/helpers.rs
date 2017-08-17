use std::cmp;
use std::cmp::Ordering;
use std::str::from_utf8;
use std::collections::BinaryHeap;
use fst::automaton::Automaton;

pub trait WeightedNFA {
    type State;
    type NextStateIter: Iterator<Item=(Self::State, f64)>;
    type InputType: Copy;

    fn start(&self) -> Self::State;

    fn is_match(&self, state: &Self::State) -> bool;

    fn can_match(&self, _state: &Self::State) -> bool {
        true
    }

    fn will_always_match(&self, _state: &Self::State) -> bool {
        false
    }

    fn accept(&self, state: &Self::State, inp: Self::InputType) ->
        Self::NextStateIter;
}

pub trait DFA {
    type State;
    type InputType;

    fn start(&self) -> Self::State;

    fn is_match(&self, state: &Self::State) -> bool;

    fn can_match(&self, _state: &Self::State) -> bool {
        true
    }

    fn will_always_match(&self, _state: &Self::State) -> bool {
        false
    }

    fn accept(&self, state: &Self::State, inp: Self::InputType) -> Self::State;
}

pub struct BeamSearchAdapter<NFA: WeightedNFA> {
    pub aut: NFA,
    pub threshold: f64,
    pub beam_size: usize
}

struct AgendaItem<IterT> {
    base_weight: f64,
    extra_weight: f64,
    iter: IterT
}

fn weight<T>(item: &AgendaItem<T>) -> f64 {
    item.base_weight + item.extra_weight
}

pub fn compare_weights(w1: &f64, w2: &f64) -> Ordering {
    w1.partial_cmp(&w2).expect("Uncomparable weights found.")
}

impl<T> Ord for AgendaItem<T> {
    fn cmp(&self, other: &AgendaItem<T>) -> Ordering {
        compare_weights(&weight(other), &weight(self))
    }
}

impl<T> PartialOrd for AgendaItem<T> {
    fn partial_cmp(&self, other: &AgendaItem<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for AgendaItem<T> {
    fn eq(&self, other: &AgendaItem<T>) -> bool {
        weight(self) == weight(other)
    }
}

impl<T> Eq for AgendaItem<T> {}

impl<NFA: WeightedNFA> DFA for BeamSearchAdapter<NFA> {
    type State = Vec<(NFA::State, f64)>;
    type InputType = NFA::InputType;

    fn start(&self) -> Self::State {
        vec![(self.aut.start(), 0.0)]
    }

    fn is_match(&self, state: &Self::State) -> bool {
        state.iter().any(|&(ref state, _weight)| self.aut.is_match(state))
    }

    fn can_match(&self, state: &Self::State) -> bool {
        state.iter().any(|&(ref state, _weight)| self.aut.can_match(state))
    }

    fn will_always_match(&self, state: &Self::State) -> bool {
        state.iter().any(|&(ref state, _weight)| self.aut.will_always_match(state))
    }

    fn expand_epsilon(&self, state: &Self::State) -> Self::State {

    }

    fn accept(&self, state: &Self::State, inp: NFA::InputType) -> Self::State {
        // initialise heap
        let mut heap: BinaryHeap<AgendaItem<NFA::NextStateIter>> = state
                .iter().map(|&(ref state, weight)| {
            AgendaItem::<NFA::NextStateIter> {
                base_weight: weight,
                extra_weight: 0.0,
                iter: self.aut.accept(state, inp)
            }
        }).collect();

        // perform beam search iteration
        let mut result: Self::State = vec![];

        while let Some(mut item) = heap.pop() {
            // XXX: Should peek?
            if let Some((next_state, next_extra_weight)) = item.iter.next() {
                let next_weight = item.base_weight + next_extra_weight;
                if next_weight > self.threshold {
                    continue;
                }
                result.push((next_state, next_weight));
                if result.len() >= self.beam_size {
                    break;
                }
                heap.push(AgendaItem::<NFA::NextStateIter> {
                    extra_weight: next_extra_weight,
                    .. item
                });
            }
        }

        //result.as_mut_slice().sort_by(|&(_, w1), &(_, w2)| compare_weights(w1, w2));
        result
    }
}

pub struct EpsilonExpandingAdapter<Wrapped: DFA + ExpandEpsilon>(pub Wrapped);

pub struct DFAUtf8Adapter<Wrapped: DFA<InputType=char>>(pub Wrapped);

impl<Wrapped: DFA<InputType=char>> DFA for DFAUtf8Adapter<Wrapped> where Wrapped::State: Clone {
    type State = (Wrapped::State, Vec<u8>);
    type InputType = u8;

    fn start(&self) -> Self::State {
        (self.0.start(), Vec::with_capacity(4))
    }

    fn is_match(&self, &(ref state, ref buffer): &Self::State) -> bool {
        buffer.len() == 0 && self.0.is_match(state)
    }

    fn can_match(&self, &(ref state, ref _buffer): &Self::State) -> bool {
        self.0.can_match(state)
    }

    fn will_always_match(&self, &(ref state, ref _buffer): &Self::State) -> bool {
        self.0.will_always_match(state)
    }

    fn accept(&self, &(ref state, ref buffer): &Self::State, inp: u8) -> Self::State {
        let mut buffer = buffer.to_owned();
        buffer.push(inp);
        let chr_res;
        {
            chr_res = from_utf8(buffer.as_slice()).map(|st| st.chars().next().unwrap());
        }
        if let Ok(chr) = chr_res {
            // Should never panic since there's at least one byte in buffer, and from_utf8
            // would fail if it didn't produce at least one char
            (self.0.accept(state, chr), vec![])
        } else {
            ((*state).clone(), buffer)
        }
    }
}

pub struct AutomatonDFAAdapter<Wrapped: DFA<InputType=u8>>(pub Wrapped);

impl<Wrapped: DFA<InputType=u8>> Automaton for AutomatonDFAAdapter<Wrapped> {
    type State = Wrapped::State;

    fn start(&self) -> Self::State {
        self.0.start()
    }

    fn is_match(&self, state: &Self::State) -> bool {
        self.0.is_match(state)
    }

    fn can_match(&self, state: &Self::State) -> bool {
        self.0.can_match(state)
    }

    fn will_always_match(&self, state: &Self::State) -> bool {
        self.0.will_always_match(state)
    }

    fn accept(&self, state: &Self::State, inp: u8) -> Self::State {
        self.0.accept(state, inp)
    }
}
