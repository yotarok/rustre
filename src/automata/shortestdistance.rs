use automata::{StateMachine,Arc,Semiring};

use std::collections::{BTreeMap,LinkedList};

pub fn shortest_distance<'a, M, F, G>(
    machine: &'a M,
    arc_filter: F,
    init_state: M::State,
    weight_eq: G) -> BTreeMap<M::State, M::Weight>
    where M: StateMachine,
          F: Fn(&M::Arc,) -> bool,
          G: Fn(&M::Weight, &M::Weight)->bool {

    let mut distance = BTreeMap::<M::State, M::Weight>::new();
    distance.insert(init_state.clone(), M::Weight::one());

    let mut visit = BTreeMap::<M::State, bool>::new();
    let mut queue = LinkedList::<M::State>::new();
    queue.push_back(init_state);

    while queue.len() > 0 {
        let st = match queue.pop_front() {
            None => { break }
            Some(head) => { head }
        };

        for arc in machine.arcs(&st) {
            if ! arc_filter(& arc) {
                continue
            };

            let nst = arc.nextstate();
            let w = distance.entry(nst.clone()).or_insert(M::Weight::zero());
            let nw = w.plus(&arc.weight());
            if ! weight_eq(w, &nw) {
                *w = nw;
                let visited = visit.entry(nst.clone()).or_insert(false);
                if *visited {
                    // currently the algorithm is breadth-first,
                    // but if we change it to best-first, we need to update the queue
                    // here
                } else {
                    *visited = true;
                    queue.push_back(nst.clone());
                }
            }
        }
    }

    distance
}

#[test]
pub fn shortest_distance_test() {
    use automata::{LoadTSV};
    use automata::vector::ByteVectorFSA;

    let fst = ByteVectorFSA::load_tsv("
0	true
0	1	0	true
1	1	0	true
2	3	0	true
3	2	0	true
3	4	0	true
".trim().as_bytes());
    let dists_0 = shortest_distance(&fst, |_| { true }, 0, |ref a, ref b| { a == b });
    let dists_2 = shortest_distance(&fst, |_| { true }, 2, |ref a, ref b| { a == b });

    assert!(dists_0.keys().cloned().collect::<Vec<i64>>() == vec!(0, 1));
    assert!(dists_2.keys().cloned().collect::<Vec<i64>>() == vec!(2, 3, 4));
}


