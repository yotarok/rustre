use runner::{optimize_fsa,Runner};
use automata::{StateMachine,Arc,boolweight,FSA};
use automata::vector::{ByteVectorFSA};

use std::io::{Read,BufReader,BufRead,stdout,Write,BufWriter};

use num_traits::{NumCast,Num};

pub trait TableElement : NumCast + Num + Ord + Copy {
}

impl TableElement for i64 {
}

impl TableElement for i32 {
}

impl TableElement for i16 {
}

impl TableElement for i8 {
}

pub struct TableFSARunner<I: TableElement> {
    transition: Vec<I>,
}

impl<I: TableElement> TableFSARunner<I> {
    #[allow(dead_code)]
    pub fn new<M: FSA<Weight=boolweight, Label=u8>>(m: M) -> TableFSARunner<I> {
        let optfsa = optimize_fsa(m);
        TableFSARunner::new_with_optimized_fsa(optfsa)
    }

    pub fn new_with_optimized_fsa(optfsa: ByteVectorFSA) -> TableFSARunner<I> {
        let nstates = optfsa.nstates().expect("Number of states should be known");

        let mut trans = Vec::<I>::new();
        let mut finals = Vec::new();

        for stidx in 0..nstates {
            trans.extend_from_slice(&[I::zero(); 256]);
            let st = stidx as i64;
            finals.push(optfsa.final_weight(&st));

            for arc in optfsa.arcs(&st) {
                let mut next = arc.nextstate();
                let lab = arc.label() as usize;
                if optfsa.final_weight(&next) {
                    next = - next;
                }
                trans[(stidx * 256) + lab] = I::from(next).unwrap();
            }
        }

        TableFSARunner {
            transition: trans,
        }
    }
}


impl<R: Read, I: TableElement> Runner<R> for TableFSARunner<I> {
    fn run(&mut self, input: R) {
        let out = stdout();
        let mut out = BufWriter::new(out.lock());
        let mut input = BufReader::with_capacity(8 * 1024, input);
        let mut l = String::new();

        while input.read_line(&mut l).unwrap_or(0) > 0 {
            let mut st = I::zero();
            let mut accepted = false;

            for b in l.bytes() {
                st = self.transition[((st.to_usize().unwrap()) << 8) | (b as usize)];
                if st < I::zero() {
                    accepted = true;
                    st = I::zero() - st;
                }
            }

            if accepted {
                write!(out, "{}", l).expect("Write error");
            }
            l.clear();
        }
    }

}
