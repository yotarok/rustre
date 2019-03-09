use automata::{StateMachine,FSA,Semiring,Arc,SimpleArc,Label,State};
use automata::vector::VectorFSA;
use automata::lazy::ArcCache;

/// Trait for composition filter state machine
pub trait CompositionFilter<LA: Arc, RA: Arc, W: Semiring> {
    type State: State;

    /// Transit filtering state machine
    ///
    /// Return None if the filter state reached to bottom.
    fn transit(la: LA, ra: RA, fs: Self::State) -> Option<(LA, RA, Self::State)>;

    fn init_state(&self) -> Self::State;
    fn final_weight(&self, s: &Self::State) -> W;
}

/// Trait for arc matcher
pub trait Matcher<LA, RA>
    where LA: Arc,
          RA: Arc {
    type InnerLeftLabel = LA::Label;
    type InnerRightLabel: Label;
    type OutputLabel: Label;

    type MatchIterator: Iterator<Item=(LA,RA)>;

    fn find_match<LI, RI>(li: LI, ri: RI) -> Self::MatchIterator
        where LI: Iterator<Item=LA>, RI: Iterator<Item=RA>;
}

/// Composition state, i.e. triple of left state, right state and filter state
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
pub struct CompositeState<L, R, FS> {
    left: L,
    right: R,
    filterstate: FS
}

impl<L: State, R: State, FS: State> State for CompositeState<L, R, FS> {
}

/// Composition state machine with generic matcher and generic filter
pub struct CompositeStateMachine<L, R, M, F>
    where L: StateMachine,
          R: StateMachine<Weight=L::Weight>,
          M: Matcher<L::Arc, R::Arc>,
          F: CompositionFilter<L::Arc, R::Arc, L::Weight> {
    left: L,
    right: R,
    matcher: M,
    filter: F,
    cache: ArcCache<CompositeState<L::State, R::State, F::State>,
                    SimpleArc<CompositeState<L::State, R::State, F::State>,
                              L::Weight, M::OutputLabel>>

}

impl<L, R, M, F> StateMachine for CompositeStateMachine<L, R, M, F>
    where L: StateMachine<Label=M::InnerLeftLabel>,
          R: StateMachine<Label=M::InnerRightLabel, Weight=L::Weight>,
          M: Matcher<L::Arc, R::Arc>,
          F: CompositionFilter<L::Arc, R::Arc, L::Weight> {

    type State = CompositeState<L::State, R::State, F::State>;
    type Weight = L::Weight;
    type Label = M::OutputLabel;
    type Arc = SimpleArc<Self::State, Self::Weight, Self::Label>;

    fn init_state(&self) -> Self::State {
        CompositeState {
            left: self.left.init_state(),
            right: self.right.init_state(),
            filterstate: self.filter.init_state()
        }
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        let lfw = self.left.final_weight(&s.left);
        let rfw = self.right.final_weight(&s.right);
        let ffw = self.filter.final_weight(&s.filterstate);
        lfw.times(&rfw.times(&ffw))
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>> {
        self.cache.query(s, &|s| {
            panic!("not implemented")
        })
    }
}

impl<L, R, M, F> FSA for CompositeStateMachine<L, R, M, F>
    where L: StateMachine<Label=M::InnerLeftLabel>,
          R: StateMachine<Label=M::InnerRightLabel, Weight=L::Weight>,
          M: Matcher<L::Arc, R::Arc>,
          F: CompositionFilter<L::Arc, R::Arc, L::Weight> {
    fn nstates(&self) -> Option<usize> { None }
}

impl<L, R, M, F> CompositeStateMachine<L, R, M, F>
    where L: StateMachine<Label=M::InnerLeftLabel>,
          R: StateMachine<Label=M::InnerRightLabel, Weight=L::Weight>,
          M: Matcher<L::Arc, R::Arc>,
          F: CompositionFilter<L::Arc, R::Arc, L::Weight> {
    fn new(left: L, right: R, matcher: M, filter: F) -> Self {
        CompositeStateMachine {
            left: left,
            right: right,
            matcher: matcher,
            filter: filter,
            cache: ArcCache::new()
        }
    }
}

pub fn compose<L, R>(left: L, right: R)
                     -> VectorFSA<L::Weight, L::Label>
    where L: StateMachine,
          R: StateMachine {
    panic!("Not implemented")
}


#[test]
pub fn compose_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	1	(1,2)	true
0	2	(1,3)	true
1	true
1	1	(3,4)	true
2	true
".trim().as_bytes());
    let fst_b = ByteVectorFSA::load_tsv("
0	1	(2,1)	true
0	2	(3,3)	true
1	2	(4,2)	true
2	2	(4,5)	true
2	true".trim().as_bytes());

        let fst_ab_src = "
0	1	(1,1)	true
0	2	(1,3)	true
1	3	(3,2)	true
2	true
3	true
3	3	(3,5)	true
".trim();

    let fst_ab = compose(fst_a, fst_b);
    let mut dump_buf = Vec::<u8>::new();
    fst_ab.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!(" === Result[Compose] ===\n{}", dumped);
    assert!(dumped.trim() == fst_ab_src);
}
