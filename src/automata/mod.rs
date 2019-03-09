pub mod vector;
pub mod lazy;
pub mod concat;
pub mod arcsort;
pub mod rmeps;
pub mod shortestdistance;
pub mod union;
pub mod closure;
pub mod determinize;
pub mod connect;
pub mod reverse;
pub mod minimize;
pub mod compose;

use std::io::{Write,Read};
use std::collections::{LinkedList,BTreeSet};

use num_traits::{Float};

/// Alias for `i64` when it is used as a state id
#[allow(non_camel_case_types)]
pub type i64state = i64;

/// Alias for `bool` when it is used as a weight semiring
#[allow(non_camel_case_types)]
pub type boolweight = bool;

#[derive(Clone,PartialOrd,PartialEq)]
pub struct Tropical<T>(T);

pub trait DumpTSV {
    fn dump_tsv(&self, dest: &mut Write);
}

pub trait LoadTSV<M> {
    fn load_tsv<R: Read>(source: R) -> M;
}

/// Trait for state descriptor in `StateMachine`
///
/// State is expected to be cheap to clone, and it is typically immutable.
/// Therefore, for some state descriptors, e.g. as in on-the-fly determinization
/// FSA, it is recommended to use reference-counted pointer for avoiding
/// expensive clone operation.
pub trait State : Ord + Sized + Clone {
}

impl State for i64state {
}

/// Trait for labels that defines constants for special labels (eps/ phi etc.)
pub trait Label : Sized + Clone + Eq {
    fn epsilon() -> Self;
}

impl<A: Label, B: Label> Label for (A, B) {
    fn epsilon() -> Self { (A::epsilon(), B::epsilon()) }
}

pub trait IOLabel : Label {
    type ILabel: Label;
    type OLabel: Label;
};

/// Trait for arcs in state machines
///
/// Arc in this library is intended to be immutable by default. When some fields
/// need to be updated, the library prefers to make an updated copy with consuming
/// the old version.
pub trait Arc :  Sized + Clone {
    type State: State;
    type Label: Label;
    type Weight: Semiring;

    fn nextstate(&self) -> Self::State;
    fn label(&self) -> Self::Label;
    fn weight(&self) -> Self::Weight;

    fn update_nextstate(self, ns: Self::State) -> Self;
}

/// Trait for weights in state machines
pub trait Semiring : PartialEq + Sized + Eq + Clone {
    fn plus(&self, _rhs: &Self) -> Self;
    fn times(&self, _rhs: &Self) -> Self;
    fn zero() -> Self;
    fn one() -> Self;

    fn is_nonzero(&self) -> bool {
        *self != Self::zero()
    }
}

impl Semiring for boolweight {
    fn plus(&self, rhs: &Self) -> Self {
        self | rhs
    }
    fn times(&self, rhs: &Self) -> Self {
        self & rhs
    }
    fn zero() -> Self {
        false
    }
    fn one() -> Self {
        true
    }
}

impl<T: Float> Eq for Tropical<T> {
}

impl<T: Float> Semiring for Tropical<T> {
    fn plus(&self, rhs: &Self) -> Self {
        Tropical(T::min(self.0, rhs.0))
    }
    fn times(&self, rhs: &Self) -> Self {
        Tropical(self.0 + rhs.0)
    }
    fn zero() -> Self {
        Tropical(T::infinity())
    }
    fn one() -> Self {
        Tropical(T::zero())
    }
}


#[derive(Clone,PartialOrd,PartialEq,Ord,Eq,Debug)]
pub struct DivByZeroError;

/// Trait for weakly-left-divisiblity of weights
///
/// It doesn't require a type to satisfy `Semiring`, but the typical usage
/// will be combined with Semiring trait
pub trait WeakLeftDiv : Sized {
    fn leftdiv(&self, denom: &Self) -> Result<Self, DivByZeroError>;
}

impl WeakLeftDiv for boolweight {
    fn leftdiv(&self, denom: &Self) -> Result<Self, DivByZeroError> {
        if ! denom {
            Err(DivByZeroError)
        } else {
            Ok(self.clone())
        }
    }
}

/// Generic base trait for graph-based state machines
///
/// The trait is designed to be generic enough for represent both
/// finite-state automata, and pushdown automata.
pub trait StateMachine {
    type State: State;
    type Arc: Arc<Weight=Self::Weight, State=Self::State, Label=Self::Label>;
    type Weight: Semiring;
    type Label: Label;

    fn init_state(&self) -> Self::State;

    fn states<'a>(&'a self) -> Box<'a + Iterator<Item=Self::State>> {
        box StateIterator::new(self)
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight;

    /// returns iterator for final states and those weights
    ///
    /// The default implementation just filters the result of `states`, which
    /// typically requires to complete state expansion; However, some type may
    /// provide more efficient implementation.
    fn final_states<'a>(&'a self)
                        -> Box<'a + Iterator<Item=(Self::State, Self::Weight)>> {
        // TO DO: Can we get rid of this temporary vector?
        let v: Vec<_> = self.states().filter_map(|s| {
            let fw = self.final_weight(&s);
            if fw == Self::Weight::zero() {
                None
            } else {
                Some((s, fw))
            }
        }).collect();
        box v.into_iter()
    }

    fn arcs<'a>(&'a self, s: &Self::State) -> Box<'a + Iterator<Item=Self::Arc>>;

}

/// Traits for mutable state machines
///
/// For mutating a state machine, two condition must be met: 1. the variable
/// containing the state machine must be mutable, and 2. the state machine must
/// be stored in modifiable representation.
/// The trait is for representing the second condition.
pub trait MutableStateMachine : StateMachine {
    fn add_new_state(&mut self) -> Self::State;
    fn add_arc(&mut self, state: &Self::State, arc: Self::Arc);
    fn set_final_weight(&mut self, s: &Self::State, w: Self::Weight);

    fn delete_states<I>(&mut self, iter: I) where I: Iterator<Item=Self::State>;
}

pub trait FSA : StateMachine {
    fn nstates(&self) -> Option<usize>;
}

#[derive(Clone,PartialOrd,PartialEq,Ord,Eq,Debug)]
pub struct SimpleArc<S: State, W: Semiring, L: Label> {
    label: L,
    weight: W,
    nextstate: S
}

impl<S: State, W: Semiring, L: Label> SimpleArc<S, W, L> {
    pub fn new(l: L, w: W, n: S) -> Self {
        SimpleArc {
            label: l,
            weight: w,
            nextstate: n
        }
    }
}

impl<S: State, W: Semiring, L: Label> Arc for SimpleArc<S, W, L> {
    type State = S;
    type Weight = W;
    type Label = L;

    fn nextstate(&self) -> Self::State {
        self.nextstate.clone()
    }
    fn label(&self) -> Self::Label {
        self.label.clone()
    }
    fn weight(&self) -> Self::Weight {
        self.weight.clone()
    }

    fn update_nextstate(self, ns: S) -> Self {
        Self {
            nextstate: ns,
            .. self
        }
    }

}

/// Generic state iterator
pub struct StateIterator<'a, M: 'a + StateMachine + ?Sized> {
    machine: &'a M,
    visited: BTreeSet<M::State>,
    queue: LinkedList<M::State>
}

impl<'a, M: StateMachine + ?Sized> Iterator for StateIterator<'a, M> where M::State : Sized {
    type Item = M::State;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front().map(|head| {
            for arc in self.machine.arcs(&head) {
                let q = arc.nextstate();
                if ! self.visited.contains(&q) {
                    self.visited.insert(q.clone());
                    self.queue.push_back(q);
                }
            }
            head
        })
    }
}

impl<'a, M: StateMachine + ?Sized> StateIterator<'a, M> {
    pub fn new(m: &'a M) -> StateIterator<'a, M> {
        let mut que = LinkedList::new();
        que.push_back(m.init_state());
        let mut visited = BTreeSet::new();
        visited.insert(m.init_state());
        StateIterator {
            machine: m,
            visited: visited,
            queue: que
        }
    }
}
