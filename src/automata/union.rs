use automata::{StateMachine,FSA,Semiring,Arc,SimpleArc,Label,State};
use automata::vector::{VectorFSA};
use automata::lazy::ArcCache;

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
pub enum UnionState<L, R> {
    Left(L),
    Right(R),
    SuperInit
}

impl<L: State, R: State> State for UnionState<L, R> {
}

pub struct UnionStateMachine<L: StateMachine, R: StateMachine<Weight=L::Weight, Label=L::Label>> {
    left: L,
    right: R,
    cache: ArcCache<UnionState<L::State, R::State>,
                    SimpleArc<UnionState<L::State, R::State>,
                              L::Weight, L::Label>>
}

impl<L: StateMachine,
     R: StateMachine<Weight=L::Weight,Label=L::Label>>
    StateMachine for UnionStateMachine<L, R>
{

    type State = UnionState<L::State, R::State>;
    type Weight = L::Weight;
    type Label = L::Label;
    type Arc = SimpleArc<Self::State, Self::Weight, Self::Label>;

    fn init_state(&self) -> Self::State {
        UnionState::SuperInit
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        match s {
            &UnionState::Left(ref s) => {
                self.left.final_weight(&s)
            }
            &UnionState::Right(ref s) => {
                self.right.final_weight(&s)
            }
            &UnionState::SuperInit => {
                Self::Weight::zero()
            }
        }
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>> {
        self.cache.query(s, &|s| {
            match s {
                &UnionState::Left(ref s) => {
                    box self.left.arcs(&s).map(|x| {
                        SimpleArc::new(x.label(), x.weight(),
                                       UnionState::Left(x.nextstate()))
                    })
                }
                &UnionState::Right(ref s) => {
                    box self.right.arcs(&s).map(|x| {
                        SimpleArc::new(x.label(), x.weight(),
                                       UnionState::Right(x.nextstate()))
                    })
                }
                &UnionState::SuperInit => {
                    let arcs = vec!(
                        SimpleArc::new(Self::Label::epsilon(), Self::Weight::one(),
                                       UnionState::Left(self.left.init_state())),
                        SimpleArc::new(Self::Label::epsilon(), Self::Weight::one(),
                                       UnionState::Right(self.right.init_state()))
                    );
                    box arcs.into_iter()
                }
            }
        })
    }
}

impl<L: StateMachine, R: StateMachine<Weight=L::Weight, Label=L::Label>> UnionStateMachine<L, R> {
    fn new(left: L, right: R) -> Self {
        UnionStateMachine {
            left: left,
            right: right,
            cache: ArcCache::new()
        }
    }
}

impl<L: FSA, R: FSA<Weight=L::Weight,Label=L::Label>> FSA for UnionStateMachine<L, R> {
    fn nstates(&self) -> Option<usize> {
        match (self.left.nstates(), self.right.nstates()) {
            (Some(l), Some(r)) => Some(l + r + 1),
            _ => None
        }
    }
}

pub fn union<L: FSA, R: FSA<Weight=L::Weight, Label=L::Label>>(left: L, right: R) ->
    VectorFSA<L::Weight, L::Label> {
    let dyn = UnionStateMachine::new(left, right);
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
1	true
1	3	0	true
2	2	1	true
2	4	2	true
3	3	1	true
3	5	2	true
4	true
4	4	3	true
5	true
5	3	0	true
5	5	3	true
".trim();

    let fst_ab = union(fst_a, fst_b);
    let mut dump_buf = Vec::<u8>::new();
    fst_ab.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!("Union Result\n====\n{}", dumped);
    assert!(dumped.trim() == fst_ab_src);
}

