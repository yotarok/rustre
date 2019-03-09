use automata::{StateMachine,FSA,Semiring,Arc,SimpleArc,Label,MutableStateMachine};
use automata::vector::{VectorFSA};
use automata::concat::ConcatStateMachine;

use std::iter::once;

pub struct ClosurePlusMachine<M: StateMachine> {
    source: M,
}

impl<M: StateMachine> StateMachine for ClosurePlusMachine<M> {
    type State = M::State;
    type Weight = M::Weight;
    type Label = M::Label;
    type Arc = SimpleArc<Self::State, Self::Weight, Self::Label>;

    fn init_state(&self) -> Self::State {
        self.source.init_state()
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        self.source.final_weight(&s)
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>> {
        let fw = self.source.final_weight(&s);
        if fw == Self::Weight::zero() {
            box self.source.arcs(&s).map(|x: M::Arc| {
                SimpleArc::new(x.label(), x.weight(),
                               x.nextstate())
            })

        } else {
            box once(SimpleArc::new(M::Label::epsilon(),
                                fw,
                                self.init_state())).chain(
                self.source.arcs(&s).map(|x: M::Arc| {
                    SimpleArc::new(x.label(), x.weight(),
                                   x.nextstate())
                })
            )
        }
    }
}

impl<M: StateMachine> ClosurePlusMachine<M> {
    fn new(src: M) -> Self {
        ClosurePlusMachine {
            source: src
        }
    }
}

impl<M: FSA> FSA for ClosurePlusMachine<M> {
    fn nstates(&self) -> Option<usize> {
        self.source.nstates()
    }
}

pub fn closure_plus<M: FSA>(src: M) -> VectorFSA<M::Weight, M::Label> {
    let dyn = ClosurePlusMachine::new(src);
    VectorFSA::<M::Weight, M::Label>::new_from_automaton(&dyn)
}

pub fn closure_star<M: FSA>(src: M) -> VectorFSA<M::Weight, M::Label> {
    let mut empty = VectorFSA::<M::Weight, M::Label>::new();
    let init = empty.add_new_state();
    empty.set_final_weight(&init, M::Weight::one());

    // This is actually a hacky solution, should implement taylored functions
    let dyn = ConcatStateMachine::new(empty, ClosurePlusMachine::new(src));
    let mut ret = VectorFSA::<M::Weight, M::Label>::new_from_automaton(&dyn);
    let ret_init = ret.init_state();
    ret.set_final_weight(&ret_init, M::Weight::one());
    ret
}

#[test]
pub fn closure_plus_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	true
0	1	0	true
1	1	1	true
1	2	2	true
2	1	0	true
2	2	3	true
2	true
".trim().as_bytes());
    let aplus_src = "
0	true
0	0	0	true
0	1	0	true
1	1	1	true
1	2	2	true
2	true
2	0	0	true
2	1	0	true
2	2	3	true
".trim();

    let fst_aplus = closure_plus(fst_a);
    let mut dump_buf = Vec::<u8>::new();
    fst_aplus.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!("A+ Result\n====\n{}", dumped);
    assert!(dumped.trim() == aplus_src);
}

#[test]
pub fn closure_star_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	true
0	1	0	true
1	1	1	true
1	2	2	true
2	1	0	true
2	2	3	true
2	true
".trim().as_bytes());
    let astar_src = "
0	true
0	1	0	true
1	true
1	1	0	true
1	2	0	true
2	2	1	true
2	3	2	true
3	true
3	1	0	true
3	2	0	true
3	3	3	true
".trim();

    let fst_aplus = closure_star(fst_a);
    let mut dump_buf = Vec::<u8>::new();
    fst_aplus.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!("A* Result\n====\n{}", dumped);
    assert!(dumped.trim() == astar_src);
}
