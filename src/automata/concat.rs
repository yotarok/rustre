use automata::{StateMachine,FSA,Semiring,Arc,SimpleArc,Label,State};
use std::iter::once;
use automata::vector::{VectorFSA};
use automata::lazy::ArcCache;

use either::{Either,Left,Right};

#[allow(unused_imports)]
use test::Bencher;

impl<L: State, R: State> State for Either<L, R> {
}

pub struct ConcatStateMachine<L: StateMachine, R: StateMachine<Weight=L::Weight, Label=L::Label>> {
    left: L,
    right: R,
    cache: ArcCache<Either<L::State, R::State>,
                    SimpleArc<Either<L::State, R::State>,
                              L::Weight, L::Label>>
}


impl<L: StateMachine,
     R: StateMachine<Weight=L::Weight,Label=L::Label>>
    StateMachine for ConcatStateMachine<L, R>
{

    type State = Either<L::State, R::State>;
    type Weight = L::Weight;
    type Label = L::Label;
    type Arc = SimpleArc<Self::State, Self::Weight, Self::Label>;

    fn init_state(&self) -> Self::State {
        Left(self.left.init_state())
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        match s {
            &Left(_) => {
                Self::Weight::zero()
            }
            &Right(ref s) => {
                self.right.final_weight(&s)
            }
        }
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>> {
        self.cache.query(s, &|s| {
            match s {
                &Left(ref s) => {
                    let fw = self.left.final_weight(&s);
                    let ret: Box<'a + Iterator<Item=Self::Arc>>= if fw == Self::Weight::zero() {
                        box self.left.arcs(&s).map(
                            |x: L::Arc| {
                                SimpleArc::new(x.label(), x.weight(),
                                               Left(x.nextstate()))
                            }
                        )
                    } else {
                        box once(
                            SimpleArc::new(L::Label::epsilon(), fw,
                                           Right(self.right.init_state()))).chain(
                            self.left.arcs(&s).map(
                                |x: L::Arc| {
                                SimpleArc::new(x.label(), x.weight(),
                                               Left(x.nextstate()))
                            }
                        ))
                    };
                    ret
                }
                &Right(ref s) => {
                    box self.right.arcs(&s).map(
                        |x: R::Arc| {
                            SimpleArc::new(x.label(), x.weight(),
                                           Right(x.nextstate()))
                        }
                    )
                }
            }
        })
    }


}

impl<L: StateMachine, R: StateMachine<Weight=L::Weight, Label=L::Label>> ConcatStateMachine<L, R> {
    pub fn new(left: L, right: R) -> Self {
        ConcatStateMachine {
            left: left,
            right: right,
            cache: ArcCache::new()
        }
    }
}

impl<L: FSA, R: FSA<Weight=L::Weight,Label=L::Label>> FSA for ConcatStateMachine<L, R> {
    fn nstates(&self) -> Option<usize> {
        match (self.left.nstates(), self.right.nstates()) {
            (Some(l), Some(r)) => Some(l + r),
            _ => None
        }
    }
}

pub fn concat<L: FSA, R: FSA<Weight=L::Weight, Label=L::Label>>(left: L, right: R) ->
    VectorFSA<L::Weight, L::Label> {
    let dyn = ConcatStateMachine::new(left, right);
    VectorFSA::<L::Weight, L::Label>::new_from_automaton(&dyn)
}

#[test]
pub fn concat_test() {
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
    let fst_b = ByteVectorFSA::load_tsv("
0	0	1	true
0	1	2	true
1	1	3	true
1	true".trim().as_bytes());

    let fst_ab_src = "
0	1	0	true
0	2	0	true
1	1	1	true
1	3	2	true
2	2	1	true
2	4	2	true
3	true
3	3	3	true
4	1	0	true
4	2	0	true
4	4	3	true
".trim();

    let fst_ab = concat(fst_a, fst_b);
    let mut dump_buf = Vec::<u8>::new();
    fst_ab.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    assert!(dumped.trim() == fst_ab_src);
}

#[bench]
pub fn concat_bench(b: &mut Bencher) {
    use automata::{LoadTSV};
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
    let fst_b = ByteVectorFSA::load_tsv("
0	0	1	true
0	1	2	true
1	1	3	true
1	true".trim().as_bytes());

    b.iter(|| {
        concat(fst_a.clone(), fst_b.clone())
    })
}
