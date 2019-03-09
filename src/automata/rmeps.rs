use automata::{StateMachine,FSA,Semiring,SimpleArc,Label,State,Arc};
use automata::vector::VectorFSA;
use automata::lazy::ArcCache;
use automata::shortestdistance::{shortest_distance};

use std::iter;
use std::rc::Rc;
use std::collections::{BTreeMap};

#[allow(unused_imports)]
use test::Bencher;

#[derive(Clone,Ord,Eq,PartialOrd,PartialEq,Debug)]
pub struct RmEpsState<S: State + Ord, W: Semiring + Ord> {
    eps_closure: Rc<BTreeMap<S, W>>,
}

impl<S: State + Ord, W: Semiring + Ord> State for RmEpsState<S, W> {
}

pub struct RmEpsStateMachine<M: StateMachine> where M::State : Ord, M::Weight : Ord {
    is_eps: Box<Fn<(M::Label,), Output=bool>>,
    source: M,
    cache: ArcCache<RmEpsState<M::State, M::Weight>,
                    SimpleArc<RmEpsState<M::State, M::Weight>,
                              M::Weight, M::Label>>
}

impl<M: StateMachine> StateMachine for RmEpsStateMachine<M>  where M::State : Ord, M::Weight : Ord {
    type State = RmEpsState<M::State, M::Weight>;
    type Weight = M::Weight;
    type Label = M::Label;
    type Arc = SimpleArc<Self::State, Self::Weight, Self::Label>;

    fn init_state(&self) -> Self::State {
        let closure =
            shortest_distance(&self.source, |ref a| { (self.is_eps)(a.label()) },
                              self.source.init_state(),
                              |ref a, ref b| { a == b });
        RmEpsState {
            eps_closure: Rc::new(closure)
        }
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        let mut ret = M::Weight::zero();
        for (ref k, ref v) in s.eps_closure.iter() {
            ret = ret.plus(& v.times(& self.source.final_weight(&k)));
        }
        ret
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>> {
        self.cache.query(s, &|s| {
            let mut chained: Box<'a + Iterator<Item=Self::Arc>> = box iter::empty();
            for (ref k, ref v) in s.eps_closure.iter() {
                let mapped = self.source.arcs(&k).filter_map(|a| {
                    if (self.is_eps)(a.label()) {
                        None
                    } else {
                        let closure =
                            shortest_distance(&self.source,
                                              |ref a| { (self.is_eps)(a.label()) },
                                              a.nextstate(),
                                              |ref a, ref b| { a == b });
                        Some(SimpleArc::new(a.label(),
                                            v.times(&a.weight()),
                                            RmEpsState {
                            eps_closure: Rc::new(closure)
                        }))
                    }
                }).collect::<Vec<Self::Arc>>();
                chained = box chained.chain(mapped.into_iter())
            }
            chained
        })
    }

}

impl<M: StateMachine> RmEpsStateMachine<M>  where M::State : Ord, M::Weight : Ord {
    fn new(m: M, pred: Box<Fn<(M::Label,), Output=bool>>) -> Self {
        RmEpsStateMachine {
            is_eps: pred,
            source: m,
            cache: ArcCache::new()
        }
    }
}

impl<M: FSA> FSA for RmEpsStateMachine<M>  where M::State : Ord, M::Weight : Ord {
    fn nstates(&self) -> Option<usize> {
        None
    }
}

pub fn rmeps<M: FSA>(m: M) -> VectorFSA<M::Weight, M::Label>  where M::State : Ord, M::Weight : Ord {
    let dyn = RmEpsStateMachine::new(m, box |l| { l == M::Label::epsilon() });
    VectorFSA::<M::Weight, M::Label>::new_from_automaton(&dyn)
}


#[test]
pub fn rmeps_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	1	0	true
1	2	1	true
1	2	2	true
1	2	0	true
2	2	0	true
2	3	0	true
3	true
".trim().as_bytes());

    let expected_src = "
0	true
0	1	1	true
0	1	2	true
1	true
".trim();

    let result = rmeps(fst_a);
    let mut dump_buf = Vec::<u8>::new();
    result.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!(" === Result[RmEps] ===\n{}", dumped);
    assert!(dumped.trim() == expected_src);
}

#[bench]
pub fn rmeps_bench(b: &mut Bencher) {
    use automata::{LoadTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	1	0	true
1	2	1	true
1	2	2	true
1	2	0	true
2	2	0	true
2	3	0	true
3	true
".trim().as_bytes());

    b.iter(|| {
        rmeps(fst_a.clone())
    })
}
