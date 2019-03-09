use automata::{Arc,SimpleArc,boolweight,FSA,MutableStateMachine,DumpTSV};

use automata::rmeps::rmeps;
use automata::concat::concat;
use automata::determinize::determinize;
use automata::arcsort::arcsort;
use automata::minimize::minimize_unweighted;
use automata::vector::VectorFSA;
use std::env;
use std::ffi::OsStr;
use std::io;

#[allow(unused_imports)]
use test::Bencher;

pub mod table;
#[allow(dead_code)]
pub mod basic;
pub mod jit;

use std::io::Read;

fn make_head_skipper() -> VectorFSA<boolweight, u8> {
    let mut ret = VectorFSA::new();
    let init = ret.add_new_state();
    ret.set_final_weight(&init, true);
    for l in 0x00..0x100 {
        ret.add_arc(&init, SimpleArc::new(l as u8, true, init.clone()));
    }
    ret
}

fn optimize_fsa<M: FSA<Weight=boolweight, Label=u8>>(m: M) -> VectorFSA<boolweight, u8> {
    let m = concat(make_head_skipper(), m);
    //eprintln!("          Raw NFA: #States = {:?}", m.nstates());
    let m = rmeps(m);
    //eprintln!("      After RmEps: #States = {:?}", m.nstates());
    let m = determinize(m);
    //eprintln!("After Determinize: #States = {:?}", m.nstates());
    let m = minimize_unweighted(m);
    //eprintln!("   After Minimize: #States = {:?}", m.nstates());
    let m = arcsort(m, |ref x, ref y| x.label().cmp(&y.label()));
    m
}

pub trait Runner<R: Read> {
    fn run(&mut self, input: R);
}

pub fn find_best_runner<M: FSA<Weight=boolweight, Label=u8>, R: Read>(m: M, use_jit: bool)
                                                                      -> Box<Runner<R>> {
    let optfsa = optimize_fsa(m);

    let empty = OsStr::new("").to_os_string();
    if env::var_os("RUSTRE_DUMP_OPTFSA").unwrap_or(empty.clone()).len() != 0 {
        optfsa.dump_tsv(&mut io::stderr());
    }

    let nst = optfsa.nstates().unwrap();
    if use_jit {
        box jit::JITFSARunner::new_with_optimized_fsa(optfsa)
    } else {
        if nst < 0x80 {
            box table::TableFSARunner::<i8>::new_with_optimized_fsa(optfsa)
        }
        else if nst < 0x8000 {
            box table::TableFSARunner::<i16>::new_with_optimized_fsa(optfsa)
        }
        else if nst < 0x80000000 {
            box table::TableFSARunner::<i32>::new_with_optimized_fsa(optfsa)
        }
        else {
            box table::TableFSARunner::<i64>::new_with_optimized_fsa(optfsa)
        }
    }
}

#[bench]
pub fn compile_rexp_bench(b: &mut Bencher) {
    use rexp::compile_rexp_nfa;
    //let rexp = "(([02468][13579]){5})+((A[02468]B[13579]C){5})+";
    let rexp = "(([02468][13579]){5})+";
    let nfa = compile_rexp_nfa(rexp);
    b.iter(|| {
        optimize_fsa(nfa.clone())
    })
}
