use std::cmp::{Eq, Ordering};
use std::hash::{Hash};
use std::str::from_utf8;
use std::collections::{BinaryHeap, HashSet};
use std::iter::Iterator;
use std::f64;

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

pub trait FollowEpsilonNFA : WeightedNFA {
    fn follow_epsilon(&self, state: &Self::State) -> Self::NextStateIter;
}

pub struct BeamSearchAdapter<NFA: WeightedNFA> where NFA::State: Eq + Hash {
    pub aut: NFA,
    pub threshold: f64,
    pub beam_size: usize
}

struct AgendaItem<IterT: Iterator> {
    base_weight: f64,
    peek: Option<IterT::Item>,
    iter: IterT,
}

impl<IterT: Iterator> AgendaItem<IterT> 
        where IterT::Item: Clone {
    fn new(base_weight: f64, mut iter: IterT) -> AgendaItem<IterT> {
        AgendaItem {
            base_weight: base_weight,
            peek: iter.next(),
            iter: iter,
        }
    }

    fn next(&mut self) -> Option<IterT::Item> {
        let old_peek = self.peek.to_owned();
        self.peek = self.iter.next();
        old_peek
    }
}

fn weight<S, IterT: Iterator<Item=(S, f64)>>(item: &AgendaItem<IterT>) -> f64 {
    item.peek.as_ref().map(|&(_, next_weight)| {
        item.base_weight + next_weight
    }).unwrap_or(f64::INFINITY)
}

pub fn compare_weights(w1: &f64, w2: &f64) -> Ordering {
    w1.partial_cmp(&w2).expect("Uncomparable weights found.")
}

impl<S, IterT: Iterator<Item=(S, f64)>> Ord for AgendaItem<IterT> {
    fn cmp(&self, other: &AgendaItem<IterT>) -> Ordering {
        compare_weights(&weight(other), &weight(self))
    }
}

impl<S, IterT: Iterator<Item=(S, f64)>> PartialOrd for AgendaItem<IterT> {
    fn partial_cmp(&self, other: &AgendaItem<IterT>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S, IterT: Iterator<Item=(S, f64)>> PartialEq for AgendaItem<IterT> {
    fn eq(&self, other: &AgendaItem<IterT>) -> bool {
        weight(self) == weight(other)
    }
}

impl<S, IterT: Iterator<Item=(S, f64)>> Eq for AgendaItem<IterT> {}

type Agenda<NFA: WeightedNFA> = BinaryHeap<AgendaItem<NFA::NextStateIter>>;
//type ExtraExpand<NFA: WeightedNFA, S> = Fn(&mut Agenda<NFA>, S, f64) -> ();

impl<NFA: WeightedNFA> BeamSearchAdapter<NFA> where NFA::State: Eq + Hash + Clone {
    fn step<ExtraExpand>(&self, state: &<Self as DFA>::State, inp: NFA::InputType,
            extra_expand: ExtraExpand) -> <Self as DFA>::State
                where ExtraExpand: Fn(&mut Agenda<NFA>, &NFA::State, f64) -> () {
        // initialise heap
        let heap: Agenda<NFA> = state
                .iter().map(|&(ref nfa_state, weight)| {
            AgendaItem::new(
                weight,
                self.aut.accept(nfa_state, inp),
            )
        }).collect();

        self.step_inner(extra_expand, heap, HashSet::new(), vec![])
    }

    fn step_inner<ExtraExpand>(&self,
                  extra_expand: ExtraExpand,
                  mut heap: Agenda<NFA>,
                  mut seen_states: HashSet<NFA::State>,
                  mut result: <Self as DFA>::State)
                        -> <Self as DFA>::State
                where ExtraExpand: Fn(&mut Agenda<NFA>, &NFA::State, f64) -> () {
        while let Some(mut item) = heap.pop() {
            let next_weight = weight(&item);
            if let Some((next_state, _)) = item.next() {
                //println!("State: {:?} {}", next_state, next_weight);
                // filter threshold
                if next_weight > self.threshold ||
                        next_weight == f64::INFINITY {
                    continue;
                }
                // filter states already in result set
                if !seen_states.contains(&next_state) {
                    seen_states.insert(next_state.clone());
                    //println!("Got result {:?}", next_state);
                    result.push((next_state.clone(), next_weight));
                    // filter by beam
                    if result.len() >= self.beam_size {
                        break;
                    }
                    // maybe expand epsilons
                    extra_expand(&mut heap, &next_state, next_weight);
                }
                // may have more edges, put back
                heap.push(AgendaItem::<NFA::NextStateIter> {
                    .. item
                });
            }
        }

        //result.as_mut_slice().sort_by(|&(_, w1), &(_, w2)| compare_weights(w1, w2));
        result
    }
}

impl<NFA: WeightedNFA> DFA for BeamSearchAdapter<NFA> where NFA::State: Eq + Hash + Clone {
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

    fn accept(&self, state: &Self::State, inp: NFA::InputType) -> Self::State {
        self.step(state, inp, |_, _, _| {})
    }
}

pub struct EpsilonExpandingBeamSearchAdapter
    <Wrapped: WeightedNFA + FollowEpsilonNFA>(pub BeamSearchAdapter<Wrapped>)
    where Wrapped::State: Eq + Hash + Clone;


impl<Wrapped: WeightedNFA + FollowEpsilonNFA> EpsilonExpandingBeamSearchAdapter<Wrapped>
        where Wrapped::State: Eq + Hash + Clone {
    fn expand_epsilon(&self, heap: &mut Agenda<Wrapped>,
                      next_state: &Wrapped::State, next_weight: f64) {
        heap.push(AgendaItem::new(
            next_weight,
            self.0.aut.follow_epsilon(next_state),
        ));
    }
}

impl<Wrapped: WeightedNFA + FollowEpsilonNFA> DFA for EpsilonExpandingBeamSearchAdapter<Wrapped> where Wrapped::State: Eq + Hash + Clone {
    type State = <BeamSearchAdapter<Wrapped> as DFA>::State;
    type InputType = <BeamSearchAdapter<Wrapped> as DFA>::InputType;

    fn start(&self) -> Self::State {
        let start_state = self.0.start();
        let (ref state, weight) = start_state[0];
        let mut heap: Agenda<Wrapped> = BinaryHeap::new();

        self.expand_epsilon(&mut heap, state, weight);

        let mut seen = HashSet::new();
        seen.insert(state.to_owned());
        let expanded_state = self.0.step_inner(
            |heap, next_state, next_weight|
                self.expand_epsilon(heap, next_state, next_weight),
            heap, seen, vec![(state.to_owned(), weight)]);
        expanded_state
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

    fn accept(&self, state: &Self::State, inp: Wrapped::InputType)
            -> Self::State {
        self.0.step(state, inp, |heap, next_state, next_weight| {
            self.expand_epsilon(heap, next_state, next_weight)
        })
    }
}

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
