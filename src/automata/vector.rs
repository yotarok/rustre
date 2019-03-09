use automata::{StateMachine,FSA,Semiring,Arc,SimpleArc,Label,i64state,boolweight,MutableStateMachine,DumpTSV,LoadTSV,State};

use std::collections::{BTreeMap,BTreeSet};
use std::io::{Write,Read,BufReader,BufRead};
use std::str::FromStr;
use std::fmt::{Display,Debug};
use std::cmp;

#[derive(Clone)]
pub struct VectorFSA<W: Semiring, L: Label> {
    arcs: Vec<Vec<SimpleArc<i64state, W, L>>>,
    finals: Vec<W>,
}

impl<L: Label, W: Semiring> VectorFSA<W, L> {
    /// Constructs a new empty VectorFSA
    pub fn new() -> Self {
        VectorFSA {
            arcs: Vec::new(),
            finals: Vec::new()
        }
    }

    /// Constructs a new VectorFSA from the given raw data
    pub fn with_data_unchecked(arcs: Vec<Vec<SimpleArc<i64state, W, L>>>,
                               finals: Vec<W>) -> Self {
        VectorFSA {
            arcs: arcs,
            finals: finals
        }
    }

    /// Constructs a new VectorFSA from the given state machine
    pub fn new_from_automaton<S: State, A: Arc<State=S, Weight=W, Label=L>>(src: &StateMachine<State=S, Label=L, Weight=W, Arc=A>) -> Self {
        let mut statemap = BTreeMap::new();
        let mut get_state_num = move |st: S| {
            let newst = statemap.len();
            *statemap.entry(st).or_insert(newst)
        };

        let init = get_state_num(src.init_state());
        assert!(init == 0);

        let mut finals = Vec::new();
        let mut arcs = Vec::new();

        for st in src.states() {
            let prev = {
                get_state_num(st.clone())
            };
            while finals.len() <= prev {
                finals.push(W::zero());
            }
            while arcs.len() <= prev {
                arcs.push(Vec::new());
            }
            finals[prev] = src.final_weight(&st);

            for arc in src.arcs(&st) {
                let next = get_state_num(arc.nextstate());
                let newarc = SimpleArc::new(arc.label(), arc.weight(), next as i64state);
                arcs[prev].push(newarc);
            }
        }

        VectorFSA {
            arcs: arcs,
            finals: finals
        }
    }

    pub fn arcs_vec<'a>(&'a self, s: &i64state) -> &'a Vec<SimpleArc<i64state, W, L>> {
        &self.arcs[*s as usize]
    }
}

impl<L: Display + Label, W: Display + Semiring> DumpTSV for VectorFSA<W, L> {
    fn dump_tsv(&self, dest: &mut Write) {
        for prev in self.states() {
            let fw = self.final_weight(&prev);
            if fw != W::zero() {
                let line = format!("{}\t{}\n", prev, fw);
                dest.write_all(line.as_bytes()).expect("Dump failed [final state]");
            }
            for arc in self.arcs(&prev) {
                let line = format!("{}\t{}\t{}\t{}\n", prev,
                                   arc.nextstate(), arc.label(), arc.weight());
                dest.write_all(line.as_bytes()).expect("Dump failed [arc]");
            }
        }
    }
}

impl<L: FromStr + Label + Debug, W: FromStr + Semiring + Debug> LoadTSV<VectorFSA<W, L>> for VectorFSA<W, L> {
    fn load_tsv<R: Read>(src: R) -> VectorFSA<W, L> {
        let mut ret = VectorFSA::new();
        let bufread = BufReader::new(src);
        for line_or_err in bufread.lines() {
            let mut line = line_or_err.expect("Read error");
            line = line.trim().to_string();

            let vals: Vec<String> = line.split_whitespace().map(|s| { s.to_string() }).collect();
            if vals.len() == 2 { // final
                match (i64::from_str(&vals[0]), W::from_str(&vals[1])) {
                    (Ok(st), Ok(w)) => {
                        while ret.nstates().unwrap() <= (st as usize) {
                            ret.add_new_state();
                        }
                        ret.set_final_weight(&st, w);
                    }
                    _ => {
                        panic!("Parse error (final)")
                    }
                }
            } else {
                assert!(vals.len() == 4, "Dumped TSV must have 2 or 4 columns");
                match (i64::from_str(&vals[0]), i64::from_str(&vals[1]),
                       L::from_str(&vals[2]), W::from_str(&vals[3])) {
                    (Ok(p), Ok(q), Ok(l), Ok(w)) => {
                        let max_pq = cmp::max(p, q);
                        while ret.nstates().unwrap() <= (max_pq as usize) {
                            ret.add_new_state();
                        }
                        ret.add_arc(&p, SimpleArc::new(l, w, q));
                    }
                    _ => {
                        panic!("Parse error (arc)")
                    }
                }
            }
        }
        ret
    }
}

impl<L: Label, W: Semiring> StateMachine for VectorFSA<W, L> {
    type State = i64state;
    type Arc = SimpleArc<i64state, W, L>;
    type Weight = W;
    type Label = L;

    fn init_state(&self) -> i64state {
        0
    }

    fn states<'a>(&'a self) -> Box<'a + Iterator<Item=i64state>> {
        box (0..(self.arcs.len())).map(|s: usize| s as i64)
    }

    fn final_weight(&self, s: &Self::State) -> Self::Weight {
        let st = *s as usize;
        self.finals[st].clone()
    }

    fn arcs<'a>(&'a self, s: &i64state) -> Box<'a + Iterator<Item=Self::Arc>> {
        let st = *s as usize;
        box self.arcs[st].iter().cloned()
    }
}


impl<L: Label, W: Semiring> FSA for VectorFSA<W, L> {

    fn nstates(&self) -> Option<usize> {
        Some(self.arcs.len())
    }

}

impl<L: Label, W: Semiring> MutableStateMachine for VectorFSA<W, L> {

    fn add_new_state(&mut self) -> Self::State {
        let n = self.arcs.len();
        self.arcs.push(Vec::new());
        self.finals.push(W::zero());
        n as i64state
    }

    fn set_final_weight(&mut self, s: &Self::State, w: Self::Weight) {
        let st = *s as usize;
        self.finals[st] = w
    }


    fn add_arc(&mut self, state: &Self::State, arc: Self::Arc) {
        self.arcs[*state as usize].push(arc)
    }

    fn delete_states<I>(&mut self, iter: I) where I: Iterator<Item=Self::State> {
        let removeset: BTreeSet<_> = iter.collect();
        // Obtain state mapping
        let newstates = {
            let mut newstates: Vec<i64> = Vec::new();

            let mut newst = 0;
            for st in self.states() {
                if removeset.contains(&st) {
                    newstates.push(-1);
                } else {
                    newstates.push(newst);
                    newst += 1;
                }
            };
            newstates
        };

        // Remap arcs
        let mut all_newarcs: Vec<Vec<SimpleArc<i64state, W, L>>> = Vec::new();
        let mut new_finals = Vec::new();
        {
            for st in self.states() {
                if removeset.contains(&st) {
                    continue;
                }

                new_finals.push(self.finals[st as usize].clone());

                let newarcs: Vec<Self::Arc> = self.arcs[st as usize].iter().filter_map(|a| {
                    let ns = newstates[a.nextstate() as usize];
                    if ns < 0 {
                        None
                    } else {
                        Some(a.clone().update_nextstate(ns))
                    }
                }).collect();

                all_newarcs.push(newarcs);
            }
        }
        self.arcs = all_newarcs;
        self.finals = new_finals;
    }

}


#[allow(dead_code)]
pub type ByteVectorFST = VectorFSA<boolweight, (u8, u8)>;
#[allow(dead_code)]
pub type ByteVectorFSA = VectorFSA<boolweight, u8>;

impl Label for u8 {
    fn epsilon() -> u8 { 0 }
}

