use automata::{StateMachine,FSA};
use automata::vector::VectorFSA;
use automata::lazy::ArcCache;

use std::cmp::Ordering;

#[allow(unused_imports)]
use test::Bencher;

pub struct ArcSortStateMachine<M: StateMachine, F: Fn(&M::Arc, &M::Arc) -> Ordering> {
    source: M,
    cmp_func: F,
    cache: ArcCache<M::State, M::Arc>
}

impl<M: StateMachine, F: Fn(&M::Arc, &M::Arc) -> Ordering> ArcSortStateMachine<M, F> {
    fn new(src: M, cmp: F) -> Self {
        ArcSortStateMachine {
            source: src,
            cmp_func: cmp,
            cache: ArcCache::new()
        }
    }
}

impl<M: FSA, F: Fn(&M::Arc, &M::Arc) -> Ordering> FSA for ArcSortStateMachine<M, F> {
    fn nstates(&self) -> Option<usize> {
        self.source.nstates()
    }
}

impl<M, F> StateMachine for ArcSortStateMachine<M, F>
    where M: StateMachine,
          F: Fn(&M::Arc, &M::Arc) -> Ordering {
    type State = M::State;
    type Weight = M::Weight;
    type Label = M::Label;
    type Arc = M::Arc;

    fn init_state(&self) -> Self::State {
        self.source.init_state()
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        self.source.final_weight(s)
    }

    fn states<'a>(&'a self) -> Box<'a + Iterator<Item=Self::State>> {
        self.source.states()
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>> {
        self.cache.query(s, &|s| {
            let mut v: Vec<Self::Arc> = self.source.arcs(s).collect();
            v.sort_by(&self.cmp_func);
            box v.into_iter()
        })
    }
}

pub fn arcsort<M: FSA, F: Fn(&M::Arc, &M::Arc) -> Ordering>(m: M, f: F) ->
    VectorFSA<M::Weight, M::Label> {
    let dyn = ArcSortStateMachine::new(m, f);
    VectorFSA::<M::Weight, M::Label>::new_from_automaton(&dyn)
}

#[bench]
pub fn arcsort_bench(b: &mut Bencher) {
    use automata::{LoadTSV,Arc};
    use automata::vector::ByteVectorFSA;


    let fst_a = ByteVectorFSA::load_tsv("
0	1	1	true
0	1	2	true
0	1	3	true
0	2	4	true
0	2	6	true
1	3	6	true
1	3	7	true
2	4	6	true
2	4	7	true
3	true
4	true
".trim().as_bytes());

    b.iter(|| {
        arcsort(fst_a.clone(),
                |ref x, ref y| { x.label().cmp(&y.label()) })
    })
}
