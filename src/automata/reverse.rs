use automata::{Semiring,Label,SimpleArc,FSA,StateMachine,Arc};
use automata::vector::{VectorFSA};

/// Reverse the input state machine
///
/// TO DO: Generalize this function to generic state machines
pub fn reverse<W: Semiring, L: Label>(m: &VectorFSA<W, L>) -> VectorFSA<W, L> {
    let nnst = m.nstates().expect("Vector FSA must have a number of states") + 1;

    let mut revarcs = vec![Vec::new(); nnst];
    let mut revfinalws = vec![W::zero(); nnst];

    revfinalws[(m.init_state() as usize) + 1] = W::one();

    for st in m.states() {
        let fw = m.final_weight(&st);
        if fw != W::zero() {
            let rarc = SimpleArc::new(L::epsilon(), fw, st + 1);
            revarcs[0].push(rarc);
        }
        for arc in m.arcs(&st) {
            let rarc = SimpleArc::new(arc.label(), arc.weight(), st + 1);
            revarcs[arc.nextstate() as usize + 1].push(rarc);
        }
    }

    VectorFSA::with_data_unchecked(revarcs, revfinalws)
}

#[test]
pub fn reverse_test() {
    use automata::{LoadTSV,DumpTSV};
    use automata::vector::ByteVectorFSA;

    let fst_a = ByteVectorFSA::load_tsv("
0	0	1	true
0	1	1	true
1	true
1	2	2	true
1	2	3	true
2	2	4	true
2	3	4	true
3	true
3	3	6	true
".trim().as_bytes());
    let expected_src = "
0	2	0	true
0	4	0	true
1	true
1	1	1	true
2	1	1	true
3	2	2	true
3	2	3	true
3	3	4	true
4	3	4	true
4	4	6	true
".trim();

    let result = reverse(&fst_a);

    let mut dump_buf = Vec::<u8>::new();
    result.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");
    println!(" === Result[Reverse] ===\n{}", dumped);
    assert!(dumped.trim() == expected_src);
}

