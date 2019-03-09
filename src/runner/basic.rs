use runner::{optimize_fsa,Runner};
use automata::{StateMachine,Arc,boolweight,FSA};
use automata::vector::VectorFSA;

use std::io::{Read, BufReader,BufRead};

pub struct BasicFSARunner {
    fsa: VectorFSA<boolweight, u8>,
}

impl BasicFSARunner {
    pub fn new<M: FSA<Weight=boolweight, Label=u8>>(m: M) -> BasicFSARunner {
        BasicFSARunner {
            fsa: optimize_fsa(m)
        }
    }
}

impl<R: Read> Runner<R> for BasicFSARunner {
    fn run(&mut self, input: R) {
        let input = BufReader::new(input);
        for l in input.lines() {
            let mut st = self.fsa.init_state();
            let mut accepted = false;
            let l = l.expect("Read failed");
            for b in l.bytes() {
                let arcvec = self.fsa.arcs_vec(&st);
                let searchres = arcvec.as_slice().binary_search_by(|ref a| {
                    a.label().cmp(&b)
                });
                match searchres {
                    Ok(idx) => {
                        st = arcvec[idx].nextstate();
                        if self.fsa.final_weight(&st) != false {
                            accepted = true;
                            break
                        }
                    }
                    Err(_) => {
                        accepted = false;
                        break
                    }
                }
            }
            if accepted {
                println!("{}", l)
            }
        }

    }
}
