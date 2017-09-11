cpp!({
    #include <cinttypes>

    #include "hfst/HfstTransducer.h"
    #include "hfst/HfstInputStream.h"
    #include "hfst/HfstOutputStream.h"
    /*#include "hfst/implementations/HfstTransitionGraph.h"*/
    #include "hfst/implementations/HfstBasicTransducer.h"

    using namespace hfst;
    using hfst::HfstTransducer;
    using hfst::HfstInputStream;
    using hfst::HfstOutputStream;
    using hfst::implementations::HfstState;
    using hfst::implementations::HfstBasicTransducer;
    using hfst::implementations::HfstBasicTransition;
    using hfst::implementations::HfstBasicTransitions;

    #include <execinfo.h>
});

use std::os::raw::c_void;
use std::ffi::CString;
use fst::Automaton;
use adapters::{WeightedNFA, AutomatonDFAAdapter, BeamSearchAdapter, EpsilonExpandingBeamSearchAdapter, compare_weights, FollowEpsilonNFA};
use std::iter;
use std::slice;

pub struct TransducerBox {
    transducer: *mut c_void,
}

impl TransducerBox {
    pub fn from_file(filename: &str) -> Option<TransducerBox> {
        let filename_cp = CString::new(filename).unwrap();
        let filename_raw = filename_cp.into_raw();
        let transducer;
        unsafe {
            transducer = cpp!([filename_raw as "char*"] -> *mut c_void as "HfstTransducer*" {
                try {
                    HfstInputStream ins(filename_raw);
                    return new HfstTransducer(ins);
                } catch (...) {
                    return NULL;
                }
            });
            if transducer.is_null() {
                return None;
            }
            CString::from_raw(filename_raw);
        }
        Some(TransducerBox { transducer: transducer })
    }

    pub fn text_to_denoised_fsa(&self, query: &str, determinize: bool,
                                trace: bool)
            -> Option<HfstBasicTransducerBox> {
        // XXX: This might be ridiculous.
        // We go Rust string -> C string -> STL string and copy each time
        let query_cp = CString::new(query).unwrap();
        let query_raw = query_cp.into_raw();
        let err_model = self.transducer;
        let graph;
        unsafe {
            graph = cpp!([
                    query_raw as "char*",
                    err_model as "HfstTransducer*",
                    determinize as "bool",
                    trace as "bool"] -> *mut c_void as "HfstBasicTransducer*" {
                try {
                    // 1. Create automaton for query
                    if (trace) {
                        fprintf(stderr, "1. Create automaton for query\n");
                        fflush(stderr);
                    }
                    std::string query_str(query_raw);
                    HfstTokenizer tok;
                    HfstTransducer query_fsa(query_str, tok, TROPICAL_OPENFST_TYPE);
                    ImplementationType t = err_model->get_type();
                    // 2. Compose with error model
                    if (trace) {
                        fprintf(stderr, "2. Compose with error model\n");
                        fflush(stderr);
                    }
                    query_fsa.compose(*err_model);
                    // 3. Project output side
                    if (trace) {
                        fprintf(stderr, "3. Project output side\n");
                        fflush(stderr);
                    }
                    query_fsa.output_project();
                    // 4. Use n best to remove low weight outputs (could use weighted version instead...)
                    //printf("4. Use n best to remove low weight outputs (could use weighted version instead...)\n");
                    //fflush(stdout);

                    // query_fsa.n_best(n_best);

                    if (determinize) {
                        // 5. (determinize?)
                        if (trace) {
                            fprintf(stderr, "5. (determinize?)\n");
                            fflush(stderr);
                        }
                        query_fsa.determinize();
                    }
                    // 6. Convert to HfstBasicTransducer
                    if (trace) {
                        fprintf(stderr, "6. Convert to HfstBasicTransducer\n");
                        fflush(stderr);
                    }
                    HfstBasicTransducer *hbt = new HfstBasicTransducer(query_fsa);
                    return hbt;
                } catch (HfstException e) {
                    fprintf(stderr, "Exception: %s\n", e().c_str());
                    fflush(stderr);

                    return NULL;
                }
            });
            if graph.is_null() {
                return None;
            }
            CString::from_raw(query_raw);
        }
        Some(HfstBasicTransducerBox { graph: graph })
    }
}

impl Drop for TransducerBox {
    fn drop(&mut self) {
        let fst = self.transducer;
        unsafe {
            cpp!([fst as "HfstTransducer*"] {
                delete fst;
            });
        }
    }
}

pub struct HfstBasicTransducerBox {
    graph: *mut c_void
}

impl HfstBasicTransducerBox {
    // self has to be mut since C++ method not marked `const`.
    // It should be!
    pub fn write_in_att_format(&mut self, filename: &str) -> bool {
        let mut graph = self.graph;
        let filename_cp = CString::new(filename).unwrap();
        let filename_raw = filename_cp.into_raw();
        unsafe {
            let result = cpp!([filename_raw as "char*", mut graph as "HfstBasicTransducer*"] -> bool as "bool" {
                try {
                    FILE *fp = fopen(filename_raw, "w");
                    graph->write_in_att_format(fp);
                    fclose(fp);
                    return true;
                } catch (...) {
                    return false;
                }
            });
            CString::from_raw(filename_raw);
            result
        }
    }

    fn step(&self, stateno: u64, inp: Vec<u8>) -> (Vec<NextStates>, Vec<u8>) {
        let graph = self.graph;
        let input_cstr = CString::new(inp).unwrap();
        let input_ptr = input_cstr.into_raw();
        let next_states;
        let inp2;
        unsafe {
            let vecinfo = cpp!(
                    [graph as "HfstBasicTransducer*",
                     stateno as "uint64_t",
                     input_ptr as "char*"] ->
                        VectorInfo<NextStates> as "struct VectorInfo" {

                std::vector<struct NextStates> next_states_out;

                HfstBasicTransitions next_states = (*graph)[stateno];

                for (HfstBasicTransitions::const_iterator it = next_states.begin();
                     it != next_states.end();
                     it++) {
                    if (it->get_input_symbol() == input_ptr) {
                        next_states_out.push_back((struct NextStates) {
                            it->get_target_state(),
                            it->get_weight()
                        });
                    }
                }

                return ((struct VectorInfo) {
                    (unsigned int)next_states_out.size(),
                    next_states_out.empty() ?
                        NULL : &next_states_out[0]
                });
            });
            // move back
            inp2 = CString::from_raw(input_ptr).into_bytes();
            let next_states_slice = slice::from_raw_parts(
                vecinfo.ptr, vecinfo.size as usize
            );
            // convert results to vector, which involves copying...
            next_states = next_states_slice.to_vec();
        }
        (next_states, inp2)
    }

    fn get_next_state_iter(&self, next_states: Vec<NextStates>) -> <Self as WeightedNFA>::NextStateIter {
        Box::new(next_states.into_iter().map(|next_state|
            ((next_state.state as u64, vec![]), next_state.weight as f64)))
    }
}

impl Drop for HfstBasicTransducerBox {
    fn drop(&mut self) {
        let graph = self.graph;
        unsafe {
            cpp!([graph as "HfstBasicTransducer*"] {
                delete graph;
            });
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct NextStates {
    state: u32,
    weight: f32
}

#[repr(C)]
pub struct VectorInfo<T> {
    size: u32,
    ptr: *const T
}

cpp!({
    struct NextStates {
        unsigned int state;
        float weight;
    };
    struct VectorInfo {
        unsigned int size;
        struct NextStates *ptr;
    };
});

impl FollowEpsilonNFA for HfstBasicTransducerBox {
    fn follow_epsilon(&self, state: &Self::State) -> Self::NextStateIter {
        let &(stateno, ref buf) = state;
        if buf.len() != 0 {
            return Box::new(iter::empty());
        }
        let epsilon = "@_EPSILON_SYMBOL_@".as_bytes().to_vec();
        let (next_states, _buf) = self.step(stateno, epsilon);
        self.get_next_state_iter(next_states)
    }
}

impl WeightedNFA for HfstBasicTransducerBox {
    type State = (u64, Vec<u8>);
    type NextStateIter = Box<Iterator<Item=(Self::State, f64)>>;
    type InputType = u8;

    fn start(&self) -> Self::State {
        return (0, vec![]);
    }

    fn is_match(&self, state: &Self::State) -> bool {
        let &(stateno, ref buf) = state;
        if buf.len() != 0 {
            return false;
        }
        let graph = self.graph;
        unsafe {
            return cpp!([graph as "HfstBasicTransducer*", stateno as "uint64_t"] -> bool as "bool" {
                return (*graph).is_final_state(stateno);
            });
        }
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::NextStateIter {
        let &(stateno, ref buf) = state;
        let mut new_buf = buf.to_owned();
        new_buf.push(byte);
        let (next_states, new_buf) = self.step(stateno, new_buf);
        if next_states.len() == 0 {
            if new_buf.len() >= 4 {
                // XXX: No support for multichars, assume 4 bytes max since that's the max length
                // of a grapheme. The reason is otherwise beam search won't work, we could keep
                // appending to the buffer of the most promising route getting no penalty each
                // time, but there's nothing there...
                Box::new(iter::empty())
            } else {
                Box::new(iter::once(((stateno, new_buf), 0.0)))
            }
        } else {
            self.get_next_state_iter(next_states)
        }
    }
}

pub type AutStack = AutomatonDFAAdapter<
    EpsilonExpandingBeamSearchAdapter<HfstBasicTransducerBox>>;

pub fn mk_stack(aut: HfstBasicTransducerBox, threshold: f64, beam_size: usize) ->
        AutStack {
    AutomatonDFAAdapter(EpsilonExpandingBeamSearchAdapter(BeamSearchAdapter {
        aut: aut,
        threshold: threshold,
        beam_size: beam_size
    }))
}

pub fn get_weights(aut: &AutStack, result: &[u8]) -> f64 {
    let mut state = aut.start();
    for inp in result {
        state = aut.accept(&state, *inp);
    }
    let weights = state.iter().filter_map(|&(ref state, ref weight)|
        if (aut.0).0.aut.is_match(state) {
            Some(*weight)
        } else {
            None
        }
    );
    weights.min_by(compare_weights).unwrap()
}
