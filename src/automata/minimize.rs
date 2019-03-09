use automata::{StateMachine,boolweight,Arc,i64state,Label,FSA,SimpleArc,MutableStateMachine};
use automata::vector::{VectorFSA};
use automata::connect::connect;
use automata::reverse::reverse;

use std::collections::{BTreeSet,BTreeMap};

#[allow(unused_imports)]
use test::Bencher;

/// Minimize unweighted automaton
pub fn minimize_unweighted<L>(m: VectorFSA<boolweight, L>)
                              -> VectorFSA<boolweight, L>
    where L: Label + Ord {

    let mut m = m;
    type State = i64state;

    let norigstate = m.nstates().expect("#States must be known for minimization");

    connect(&mut m);

    // Initialize
    let tr = reverse(&m);

    let allstates: BTreeSet<State> = m.states().collect();
    let finals: BTreeSet<State> = m.final_states().map(|t| t.0).collect();
    let mut stack: Vec<BTreeSet<State>> = vec![finals.clone()];

    let mut partitions: Vec<BTreeSet<State>> = vec![
        finals.clone(), allstates.difference(&finals).cloned().collect()];

    while ! stack.is_empty() {
        let set: BTreeSet<State> = stack.pop().expect("Stack is empty");

        // map from prefix label and preceeding state
        let mut prevs: BTreeMap<L, BTreeSet<State>> = BTreeMap::new();

        for s in set {
            for rarc in tr.arcs(&(s + 1)) {
                prevs.entry(rarc.label())
                    .or_insert(BTreeSet::new())
                    .insert(rarc.nextstate() - 1);
            }
        }

        for (_prev_label, prev_set) in prevs {
            let mut new_partitions = Vec::new();

            for partition in partitions.iter() {
                let intersect: BTreeSet<State> = partition.intersection(&prev_set).cloned().collect();
                let diff: BTreeSet<State> = partition.difference(&prev_set).cloned().collect();
                if intersect.is_empty() || diff.is_empty() {
                    new_partitions.push(partition.clone());
                    continue;
                }

                new_partitions.push(intersect.clone());
                new_partitions.push(diff.clone());

                let mut found = false;
                for stack_elem in stack.iter_mut() {
                    if *stack_elem == *partition {
                        *stack_elem = intersect.clone();
                        found = true;
                    }
                }
                if found {
                    stack.push(diff);
                } else {
                    if intersect.len() <= diff.len() {
                        stack.push(intersect);
                    } else {
                        stack.push(diff);
                    }
                }
            }
            partitions = new_partitions;
        }
    }

    let mut init_part = None;
    for (partid, part) in partitions.iter().enumerate() {
        if part.contains(&m.init_state()) {
            init_part = Some(partid);
        }
    }
    if let Some(pidx) = init_part {
        partitions.swap(0, pidx);
    } else {
        panic!("Initial state isn't found");
    }

    let mut state2part = vec![0; norigstate];
    for (partid, part) in partitions.iter().enumerate() {
        for s in part.iter() {
            state2part[*s as usize] = partid;
        }
    }

    let mut ret = VectorFSA::new();
    for (partid, part) in partitions.iter().enumerate() {
        let st = ret.add_new_state();
        let mut is_final = false;

        let mut arcset = BTreeSet::new(); // for removing duplicates
        for os in part.iter() {
            if m.final_weight(&os) {
                is_final = true;
            }
            for arc in m.arcs(&os) {
                arcset.insert(
                    SimpleArc::new(
                        arc.label(), true,
                        state2part[arc.nextstate() as usize] as i64)
                );
            }
        }

        for arc in arcset.into_iter() {
            ret.add_arc(&(partid as i64), arc);
        }

        if is_final {
            ret.set_final_weight(&st, true);
        }
    }

    ret
}

#[test]
pub fn minimize_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	1	1	true
0	1	2	true
0	1	3	true
0	2	4	true
0	2	6	true
1	3	6	true
1	3	7	true
2	4	6	true
2	4	7	true
3	true
4	true
".trim().as_bytes());

    let expected_src = "
0	1	1	true
0	1	2	true
0	1	3	true
0	1	4	true
0	1	6	true
1	2	6	true
1	2	7	true
2	true
".trim();

    let result = minimize_unweighted(fst_a);
    let mut dump_buf = Vec::<u8>::new();
    result.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!(" === Result[Minimize] ===\n{}", dumped);
    assert!(dumped.trim() == expected_src);
}

#[bench]
pub fn minimize_bench(b: &mut Bencher) {
    use automata::{LoadTSV};
    use automata::vector::ByteVectorFSA;


    let fst_a = ByteVectorFSA::load_tsv("
0	1	1	true
0	1	2	true
0	1	3	true
0	2	4	true
0	2	6	true
1	3	6	true
1	3	7	true
2	4	6	true
2	4	7	true
3	true
4	true
".trim().as_bytes());

    b.iter(|| {
        minimize_unweighted(fst_a.clone())
    })
}
