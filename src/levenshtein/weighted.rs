use helpers::{WeightedNFA, DFA, BeamSearchAdapter, DFAUtf8Adapter,
              AutomatonDFAAdapter, compare_weights};

use std::sync::Arc;
use fst::Automaton;

pub struct WeightedLevenshteinNFA {
    query: Arc<Vec<char>>
}

impl WeightedLevenshteinNFA {
    pub fn new(query: &str) -> WeightedLevenshteinNFA {
        let mut query_chars = Vec::with_capacity(query.len());
        query_chars.extend(query.chars());
        query_chars.shrink_to_fit();
        WeightedLevenshteinNFA { query: Arc::new(query_chars) }
    }
}

enum NextStatesState {
    Match, Insert, Substitute, Delete
}

pub struct LevenshteinNextStates {
    chars: usize,
    query: Arc<Vec<char>>,
    inp: char,
    state: NextStatesState,
    extra_weight: f64,
    deleted: bool
}

impl LevenshteinNextStates {
    pub fn new(chars: usize, query: &Arc<Vec<char>>, inp: char) -> LevenshteinNextStates {
        LevenshteinNextStates {
            chars: chars,
            query: Arc::clone(query),
            inp: inp,
            state: NextStatesState::Match,
            extra_weight: 0.0,
            deleted: false
        }
    }
}

impl Iterator for LevenshteinNextStates {
    type Item = (usize, f64);

    fn next(&mut self) -> Option<(usize, f64)> {
        // I heard you like state machines...
        loop {
            match self.state {
                // match
                // query: a|bc
                // should match: a|bc
                NextStatesState::Match => {
                    self.state = NextStatesState::Substitute;
                    if self.chars < self.query.len() && self.query[self.chars] == self.inp {
                        return Some((self.chars + 1, 0.0 + self.extra_weight));
                    }
                }
                // substitution
                // query: a|bc
                // should match: a|sc
                // => increment query pointer
                // &  let to match pointer move on one step
                NextStatesState::Substitute => {
                    self.state = NextStatesState::Insert;
                    if self.chars < self.query.len() {
                        return Some((self.chars + 1, 1.0 + self.extra_weight));
                    }
                }
                // insertion
                // query: a|bc
                // should match: a|ibc
                // => leave query pointer as is
                // &  let to match pointer move on one step
                NextStatesState::Insert => {
                    self.state = NextStatesState::Delete;
                    if !self.deleted { // eliminate dupes: insertion + deletion = substitution
                        return Some((self.chars, 1.0 + self.extra_weight));
                    }
                }
                // deletion
                // query: a|bc
                // should match: a|c
                // => move query pointer forward one
                // &  recurse so as to avoid moving match pointer onwards
                NextStatesState::Delete => {
                    if self.chars + 1 >= self.query.len() {
                        // deleted last character from match, so nothing else can match
                        return None;
                    }
                    self.chars += 1;
                    self.state = NextStatesState::Match;
                    self.extra_weight += 1.0;
                    self.deleted = true;
                }
            }
        }
    }
}

// A cleaner solution would be probably be possible with associated lifetimes
impl WeightedNFA for WeightedLevenshteinNFA {
    type State = usize;
    type NextStateIter = LevenshteinNextStates;
    type InputType = char;

    fn start(&self) -> Self::State {
        0
    }

    fn is_match(&self, state: &Self::State) -> bool {
        *state == self.query.len()
    }

    fn can_match(&self, _state: &Self::State) -> bool {
        true
    }

    fn will_always_match(&self, _state: &Self::State) -> bool {
        false
    }

    fn accept(&self, state: &Self::State, inp: char) -> LevenshteinNextStates {
        LevenshteinNextStates::new(*state, &self.query, inp)
    }
}

pub type LevenshteinStack = AutomatonDFAAdapter<
    DFAUtf8Adapter<BeamSearchAdapter<WeightedLevenshteinNFA>>>;

pub fn mk_levenshtein(query: &str, threshold: f64, beam_size: usize) -> LevenshteinStack {
    AutomatonDFAAdapter(DFAUtf8Adapter(BeamSearchAdapter {
        aut: WeightedLevenshteinNFA::new(query),
        threshold: threshold,
        beam_size: beam_size
    }))
}

pub fn get_levenshtein_weights(aut: &LevenshteinStack, result: &[u8]) -> f64 {
    let mut state = aut.start();
    for inp in result {
        state = aut.accept(&state, *inp);
    }
    let weights = state.0.iter().filter_map(|&(state, weight)|
        if (aut.0).0.aut.is_match(&state) {
            Some(weight)
        } else {
            None
        }
    );
    weights.min_by(compare_weights).unwrap()
    /*
    let mut first_weight = None;
    for weight in weights {
        if first_weight.is_none() {
            first_weight = Some(weight);
        }
    }
    return first_weight.unwrap();
    */
}
