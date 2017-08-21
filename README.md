Some extra automata for https://github.com/BurntSushi/fst

What's in the box?
==================

* src/helpers.rs - (better name needed) Contains among other things a trait for weighted non deterministic (finite) automata (WNFA) and an adapter which performs beam search on WNFAs and presents the result as a deterministic (finite) automaton (DFA)

* src/levenshtein/unweighted.rs - Probably the most naive/simple Levenshtein implementation possible. In most cases probably (much) less efficient than the implementation in burntsushi/levenshtein-fst, however, burntsushi/levenshtein-fst can have cases where minimisation can take too much time/memory whereas this implementation should have quite predictable performance.

* src/levenshtein/weighted.rs - Similar but weighted so you can perform beam search on it and get a list of results in descending order of likelihood.

* src/hfst.rs - A wrapper around part of HFST which provides an automaton wrapping an error model FST used together with a query so that the automaton accepts all possible 'corrected' strings based on the query according to the error model FST. For an example of an error FST, see TODO.

How to use it
=============

No instructions yet, since this library currently exists mainly to drive
a proof of concept for TODO. In the meantime, if you're interested in using this for
something, you're welcome to use the issue tracker and I'll try and help out.
