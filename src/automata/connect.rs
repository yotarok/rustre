use automata::{StateMachine,Arc,State,Semiring,MutableStateMachine};

use std::collections::{BTreeMap,BTreeSet};

/**
 * Type for represent node color-state used while DFS visit
 */
#[derive(Clone,Debug)]
enum NodeColor {
    White,
    Grey,
    Black
}

/**
 * EventType is used for supporting DFS visit with single closure
 *
 * For accomodating multiple types of callbacks, `dfs_visit` function
 * primarily uses `DFSVisitor` trait for defining callback functions.
 * However, it is convenient if those callbacks can be defined also as
 * a single closure with type `FnMut(VisitorEvent) -> bool`.
 */
pub enum VisitorEvent<'a, 'b, S: 'a + State, A: 'b + Arc<State=S>> {
    EnterState(&'a S),
    VisitTreeArc(&'a S, &'b A),
    VisitBackArc(&'a S, &'b A),
    VisitCrossArc(&'a S, &'b A),
    ExitState(&'a S, Option<&'a S>)
}

/**
 * Trait for structs defining `dfs_visit` callback functions
 */
pub trait DFSVisitor<S: State, A: Arc<State=S>> {

    fn enter_state(&mut self, _st: &S) -> bool { true }
    fn visit_tree_arc(&mut self, _st: &S, _a: &A) -> bool { true }
    fn visit_back_arc(&mut self, _st: &S, _a: &A) -> bool { true }
    fn visit_cross_arc(&mut self, _st: &S, _a: &A) -> bool { true }
    fn exit_state(&mut self, _st: &S, _p: Option<&S>) { }
}

/**
 * Implementation of `DFSVisitor` trait for `FnMut(VisitorEvent) -> bool`
 */
impl<F, S, A> DFSVisitor<S, A> for F
    where S: State, A: Arc<State=S>, F:(FnMut(VisitorEvent<S, A>) -> bool) {
    fn enter_state(&mut self, st: &S) -> bool {
        (self)(VisitorEvent::EnterState(st))
    }
    fn visit_tree_arc(&mut self, st: &S, a: &A) -> bool {
        (self)(VisitorEvent::VisitTreeArc(st, a))
    }
    fn visit_back_arc(&mut self, st: &S, a: &A) -> bool {
        (self)(VisitorEvent::VisitBackArc(st, a))
    }
    fn visit_cross_arc(&mut self, st: &S, a: &A) -> bool {
        (self)(VisitorEvent::VisitCrossArc(st, a))
    }
    fn exit_state(&mut self, st: &S, _p: Option<&S>) {
        (self)(VisitorEvent::ExitState(st, _p));
    }
}

/**
 * DFSVisitor for finding coaccessible states from an FSA
 */
struct CoAccessFinder<'a, M> where M: 'a + StateMachine {
    machine: &'a M,
    pub access: BTreeSet<M::State>,
    pub coaccess: BTreeSet<M::State>
}

impl<'a, M> CoAccessFinder<'a, M> where M: 'a + StateMachine {
    pub fn new(m: &'a M) -> Self {
        CoAccessFinder {
            machine: m,
            access: BTreeSet::new(),
            coaccess: BTreeSet::new(),
        }
    }
}

impl<'a, M> DFSVisitor<M::State, M::Arc> for CoAccessFinder<'a, M>
    where M: 'a + StateMachine {

    fn enter_state(&mut self, st: &M::State) -> bool {
        self.access.insert(st.clone());
        true
    }

    fn visit_back_arc(&mut self, st: &M::State, a: &M::Arc) -> bool {
        let next_coaccess = self.coaccess.contains(&a.nextstate());
        if next_coaccess {
            self.coaccess.insert(st.clone());
        }
        true
    }

    fn visit_cross_arc(&mut self, st: &M::State, a: &M::Arc) -> bool {
        let next_coaccess = self.coaccess.contains(&a.nextstate());
        if next_coaccess {
            self.coaccess.insert(st.clone());
        }
        true
    }

    fn exit_state(&mut self, st: &M::State, popt: Option<&M::State>) {
        if self.machine.final_weight(st) != M::Weight::zero() {
            self.coaccess.insert(st.clone());
        }

        if let Some(p) = popt {
            self.coaccess.insert(p.clone());
        }
    }
}

/**
 * Traverse the given state machine in depth-first manner, and calls callbacks
 *
 * This function currently doesn't visit states that are not accessible
 * (from the initial state).
 */
pub fn dfs_visit<M,V,F>(m: &M,
                        mut visitor: V,
                        filter: F) -> V
    where V: DFSVisitor<M::State, M::Arc>,
          F: Fn(&M::Arc) -> bool,
          M: StateMachine,
          M::State: Ord {

    let mut state_color: BTreeMap<M::State, NodeColor> = BTreeMap::new();
    let mut state_stack: Vec<(M::State, Box<Iterator<Item=M::Arc>>)> = Vec::new();
    let start = m.init_state();

    state_color.insert(start.clone(), NodeColor::Grey); // Should it be checked?

    state_stack.push((start.clone(), m.arcs(&start).into()));
    visitor.enter_state(&start);

    while ! state_stack.is_empty() {
        let st = state_stack.last().expect("Stack is actually empty, why?").0.clone();
        let nextarc = {
            let mut arcit = &mut state_stack.last_mut().expect("Stack is actually empty, why?").1;
            arcit.next()
        };

        match nextarc {
            None => {
                state_color.insert(st.clone(), NodeColor::Black);
                state_stack.pop();

                let parent_state_opt = state_stack.last().map(|t| &t.0);

                visitor.exit_state(&st, parent_state_opt);
            }
            Some(ref a) if filter(a) => {
                let next_state = a.nextstate().clone();
                let next_color = state_color.get(&next_state)
                    .cloned().unwrap_or(NodeColor::White);

                match next_color {
                    NodeColor::White => {
                        if ! visitor.visit_tree_arc(&st, &a) {
                            break;
                        }

                        state_color.insert(next_state.clone(), NodeColor::Grey);
                        state_stack.push((next_state.clone(), m.arcs(&next_state).into()));

                        visitor.enter_state(&next_state);
                    },
                    NodeColor::Grey => {
                        visitor.visit_back_arc(&st, &a);
                    },
                    NodeColor::Black => {
                        visitor.visit_cross_arc(&st, &a);
                    }
                }
            }
            _ => { // filtered out
            }
        }

    }
    visitor
}

/**
 * Delete all states that are not accessible and co-accessble.
 */
pub fn connect<M>(m: &mut M) where M: MutableStateMachine {
    let remove: BTreeSet<_> = {
        let mut finder = CoAccessFinder::new(m);
        finder = dfs_visit(m, finder, |_| true);
        let all_states: BTreeSet<M::State> = m.states().collect();
        let connected = finder.coaccess.intersection(&finder.access).cloned().collect();
        all_states.difference(&connected).cloned().collect()
    };
    m.delete_states(remove.iter().cloned());
}

#[test]
pub fn dfs_visit_test() {
    use automata::{LoadTSV,boolweight,i64state,SimpleArc};
    use automata::vector::ByteVectorFSA;
    use self::VisitorEvent::*;
    let fst_a = ByteVectorFSA::load_tsv("
0	1	1	true
1	2	2	true
1	true
2	3	3	true
3	0	4	true
3	4	9	true
5	0	9	true
".trim().as_bytes());


    let mut log = String::new();

    dfs_visit(&fst_a, |ev: VisitorEvent<i64state, SimpleArc<i64state,boolweight,u8>>| {
        match ev {
            EnterState(ref st) => {
                log.push_str(format!("EN{}\n", st).as_str());
            },
            VisitTreeArc(ref st, ref a) => {
                log.push_str(format!(
                    "VT{},{},{}\n",
                    st, a.nextstate(), a.label()).as_str());
            },
            VisitBackArc(ref st, ref a) => {
                log.push_str(format!(
                    "VB{},{},{}\n",
                    st, a.nextstate(), a.label()).as_str());
            },
            VisitCrossArc(ref st, ref a) => {
                log.push_str(format!(
                    "VX{},{},{}\n",
                    st, a.nextstate(), a.label()).as_str());
            },
            ExitState(ref st, ref _popt) => {
                log.push_str(format!("EX{}\n", st).as_str());
            }
        };
        true
    }, |_| { true });
    println!("log = {}",log);
    assert_eq!(log.trim(), "
EN0
VT0,1,1
EN1
VT1,2,2
EN2
VT2,3,3
EN3
VB3,0,4
VT3,4,9
EN4
EX4
EX3
EX2
EX1
EX0
".trim());
}

#[test]
pub fn coaccess_finder_test() {
    use automata::{LoadTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	1	1	true
1	2	2	true
1	true
2	3	3	true
3	0	4	true
3	4	9	true
5	0	9	true
".trim().as_bytes());

    let mut finder = CoAccessFinder::new(&fst_a);

    finder = dfs_visit(&fst_a, finder, |_| { true });

    assert_eq!(finder.coaccess,
               [0, 1, 2, 3].iter().cloned().collect());

}

#[test]
pub fn connect_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;
    let mut fst_a = ByteVectorFSA::load_tsv("
0	5	1	true
5	2	2	true
5	true
2	3	3	true
3	0	4	true
3	4	9	true
1	0	9	true
".trim().as_bytes());

    let fst_b_src = "
0	3	1	true
1	2	3	true
2	0	4	true
3	true
3	1	2	true
".trim();

    connect(&mut fst_a);

    let mut dump_buf = Vec::<u8>::new();
    fst_a.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    assert_eq!(dumped.trim(), fst_b_src);

}
