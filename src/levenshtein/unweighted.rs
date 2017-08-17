use std::str::from_utf8;
use fst::automaton::Automaton;

#[derive(Debug, Clone, Copy)]
pub struct SimpleLevenshteinState {
    chars: usize,
    distance: u64,
}

fn get_next_states(state: &SimpleLevenshteinState, query: &[char], chr: char, threshold: u64) -> Vec<SimpleLevenshteinState> {
    let mut new_states = vec![];
    let have_extra_dist = state.distance < threshold;
    // insertion
    // query: a|bc
    // should match: a|ibc
    // => leave query pointer as is
    // &  let to match pointer move on one step
    if have_extra_dist {
        new_states.push(SimpleLevenshteinState {
            chars: state.chars,
            distance: state.distance + 1
        });
    }
    if state.chars < query.len() {
        // match
        // query: a|bc
        // should match: a|bc
        if query[state.chars] == chr {
            new_states.push(SimpleLevenshteinState {
                chars: state.chars + 1,
                distance: state.distance
            });
        }
        if have_extra_dist {
            // for deletion & substitution
            let forward_query_mod_state = SimpleLevenshteinState {
                chars: state.chars + 1,
                distance: state.distance + 1
            };
            // deletion
            // query: a|bc
            // should match: a|c
            // => move query pointer forward one
            // &  recurse so as to avoid moving match pointer onwards
            for next_state in get_next_states(&forward_query_mod_state, query, chr, threshold) {
                new_states.push(next_state);
            }
            // substitution
            // query: a|bc
            // should match: a|sc
            // => increment query pointer
            // &  let to match pointer move on one step
            new_states.push(forward_query_mod_state);
        }
    }
    new_states
}

impl SimpleLevenshteinState {
    fn is_match(&self, query: &[char], threshold: u64) -> bool {
        (self.chars == query.len()) && (self.distance <= threshold)
    }

    fn can_match(&self, _query: &[char], threshold: u64) -> bool {
        self.distance <= threshold
    }

    fn accept(&self, query: &[char], chr: char, threshold: u64) -> Vec<SimpleLevenshteinState> {
        get_next_states(self, query, chr, threshold)
    }
}

#[derive(Debug)]
pub struct CompoundLevenshteinState {
    states: Vec<SimpleLevenshteinState>,
    buffer: Vec<u8>
}

#[derive(Debug)]
pub struct SimpleLevenshtein {
    query: Vec<char>,
    threshold: u64
}

impl SimpleLevenshtein {
    pub fn new(query: &str, threshold: u64) -> SimpleLevenshtein {
        let query_vec = query.chars().collect();
        SimpleLevenshtein { query: query_vec, threshold: threshold }
    }
}

impl Automaton for SimpleLevenshtein {
    type State = CompoundLevenshteinState;

    fn start(&self) -> CompoundLevenshteinState {
        CompoundLevenshteinState{
            states: vec![SimpleLevenshteinState { chars: 0, distance: 0 }],
            buffer: vec![]
        }
    }

    fn is_match(&self, state: &CompoundLevenshteinState) -> bool {
        if state.buffer.len() != 0 {
            false
        } else {
            state.states.iter().any(|state| state.is_match(self.query.as_slice(), self.threshold))
        }
    }

    fn can_match(&self, state: &CompoundLevenshteinState) -> bool {
        state.states.iter().any(|state| state.can_match(self.query.as_slice(), self.threshold))
    }

    fn accept(&self, state: &CompoundLevenshteinState, byte: u8) -> CompoundLevenshteinState {
        let mut buffer = state.buffer.to_owned();
        buffer.push(byte);
        let chr_res;
        {
            chr_res = from_utf8(buffer.as_slice()).map(|st| st.chars().next().unwrap());
        }
        if let Ok(chr) = chr_res {
            // Should never panic since there's at least one byte in buffer, and from_utf8
            // would fail if it didn't produce at least one char
            CompoundLevenshteinState {
                states: state.states.iter().flat_map(|state| state.accept(self.query.as_slice(), chr, self.threshold)).collect(),
                buffer: vec![]
            }
        } else {
            CompoundLevenshteinState {
                states: state.states.to_vec(),
                buffer: buffer
            }
        }
    }
}
