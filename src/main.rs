#![feature(box_syntax, box_patterns,unboxed_closures,try_from,alloc,allocator_api,conservative_impl_trait,universal_impl_trait,test)]

extern crate either;
#[macro_use]
extern crate combine;
extern crate num_traits;
extern crate clap;
extern crate llvm_sys;
extern crate alloc;
extern crate test;

macro_rules! c_str {
    ($s:expr) => { {
        concat!($s, "\0").as_ptr() as *const i8
    } }
}

mod automata;
mod rexp;
mod runner;
mod utils;

#[allow(unused_imports)]
use std::io::stdout;

use clap::{Arg, App};
use std::fs::File;

fn main() {
    let matches = App::new("My GREPPER")
        .version("1.0")
        .author("Yotaro Kubo <yotaro@ieee.org>")
        .about("Does GREP")
        .arg(Arg::with_name("expr")
             .short("e")
             .long("expr")
             .value_name("EXPR")
             .required(true)
             .help("Regular expression")
             .takes_value(true))
        .arg(Arg::with_name("jit")
            .short("J")
            .help("Use JIT when possible"))
        .arg(Arg::with_name("INPUT")
             .help("Sets the input file to use")
             .required(true)
             .index(1))
        .get_matches();
    let fsa = rexp::compile_rexp_nfa(matches.value_of("expr").unwrap());

    let filename = matches.value_of("INPUT").unwrap();
    let use_jit = matches.occurrences_of("jit") > 0;

    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => panic!("Cannot open the file")
    };

    let mut runner = runner::find_best_runner(fsa, use_jit);

    runner.run(file);
}
