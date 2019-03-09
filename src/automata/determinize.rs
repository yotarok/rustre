use automata::{StateMachine,FSA,Semiring,SimpleArc,State,Arc,WeakLeftDiv};
use automata::vector::VectorFSA;
use automata::lazy::ArcCache;

use std::rc::Rc;
use std::collections::{BTreeMap,BTreeSet};

#[allow(unused_imports)]
use test::Bencher;

#[derive(Clone,Ord,Eq,PartialOrd,PartialEq,Debug)]
pub struct DeterminizeState<S: State + Ord, W: Semiring + Ord + WeakLeftDiv> {
    residual: Rc<BTreeMap<S, W>>
}

impl<S, W> DeterminizeState<S, W>
    where S : Ord + State, W : Ord + Semiring + WeakLeftDiv {

    fn transitions<'a, M>(&'a self, machine: &M)
                          -> Box<'a + Iterator<Item=M::Label>>
        where M : StateMachine<State=S, Weight=W>, M::Label : 'a + Ord {

        let mut ret = BTreeSet::new();
        for (ref st, ref _resw) in self.residual.iter() {
            for arc in machine.arcs(&st) {
                ret.insert(arc.label());
            }
        }
        box ret.into_iter()
    }
}

impl<S, W> State for DeterminizeState<S, W>
    where S : Ord + State, W : Ord + Semiring + WeakLeftDiv {
}

pub struct DeterminizedMachine<M: StateMachine>
    where M::State : Ord, M::Weight : Ord + WeakLeftDiv {

    source: M,
    cache: ArcCache<DeterminizeState<M::State, M::Weight>,
                    SimpleArc<DeterminizeState<M::State, M::Weight>,
                              M::Weight, M::Label>>
}

impl<M: StateMachine> StateMachine for DeterminizedMachine<M>
    where M::State : Ord,
          M::Weight : Ord + WeakLeftDiv,
          M::Label : Ord {

    type State = DeterminizeState<M::State, M::Weight>;
    type Weight = M::Weight;
    type Label = M::Label;
    type Arc = SimpleArc<Self::State, Self::Weight, Self::Label>;

    fn init_state(&self) -> Self::State {
        let mut resid = BTreeMap::new();
        resid.insert(self.source.init_state().clone(), Self::Weight::one());
        DeterminizeState {
            residual: Rc::new(resid)
        }
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        let mut ret = M::Weight::zero();
        for (ref st, ref resw) in s.residual.iter() {
            let fw = self.source.final_weight(&st);
            if fw.is_nonzero() {
                ret = ret.plus(&resw.times(&fw));
            }
        }
        ret
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>> {
        self.cache.query(s, &|s| {
            let mut ret = Vec::new();
            for l in s.transitions(&self.source) {
                // TO DO: Very slow
                let mut transw = Self::Weight::zero();
                let mut nextresid = BTreeMap::new();
                for (ref st, ref resw) in s.residual.iter() {
                    for arc in self.source.arcs(&st) {
                        if arc.label() == l {
                            transw = transw.plus(&resw.times(&arc.weight()));
                        }
                    }
                    for arc in self.source.arcs(&st) {
                        // TO DO: If there's already an entry, need to take a plus
                        if arc.label() == l {
                            let rw = resw.times(&arc.weight()).leftdiv(&transw)
                                .expect("div by zero semiring");
                            nextresid.insert(arc.nextstate(), rw);
                        }
                    }
                }
                ret.push(SimpleArc::new(l, transw, DeterminizeState {
                    residual: Rc::new(nextresid)
                }));
            }
            box ret.into_iter()
        })
    }

}

impl<M: StateMachine> DeterminizedMachine<M>
    where M::State : Ord,
          M::Weight : Ord + WeakLeftDiv,
          M::Label : Ord {

    pub fn new(m: M) -> Self {
        DeterminizedMachine {
            source: m,
            cache: ArcCache::new()
        }
    }
}

impl<M: FSA> FSA for DeterminizedMachine<M>
    where M::State : Ord,
          M::Weight : Ord + WeakLeftDiv,
          M::Label : Ord {

    fn nstates(&self) -> Option<usize> {
        None
    }
}

pub fn determinize<M: FSA>(m: M) -> VectorFSA<M::Weight, M::Label>
    where M::State : Ord,
          M::Weight : Ord + WeakLeftDiv,
          M::Label : Ord {

    let dyn = DeterminizedMachine::new(m);
    VectorFSA::<M::Weight, M::Label>::new_from_automaton(&dyn)
}


#[test]
pub fn determinize_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	1	1	true
0	2	1	true
1	3	3	true
1	3	4	true
2	3	4	true
3	true
".trim().as_bytes());

    let expected_src = "
0	1	1	true
1	2	3	true
1	2	4	true
2	true
".trim();

    let result = determinize(fst_a);
    let mut dump_buf = Vec::<u8>::new();
    result.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!(" === Result[Determinize] ===\n{}", dumped);
    assert!(dumped.trim() == expected_src);
}

#[bench]
pub fn determinize_bench(b: &mut Bencher) {
    use automata::{LoadTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	1	1	true
0	2	1	true
1	3	3	true
1	3	4	true
2	3	4	true
3	true
".trim().as_bytes());

    b.iter(|| {
        determinize(fst_a.clone())
    })
}
