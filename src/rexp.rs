use automata::{MutableStateMachine,SimpleArc,Semiring,boolweight};
use automata::vector::ByteVectorFSA;
use automata::concat::concat;
use automata::rmeps::rmeps;
use automata::closure::{closure_plus, closure_star};
use automata::union::union;

use std::iter::once;
use std::convert::TryFrom;

use combine::{many1,sep_by,Stream,one_of,none_of,try,optional,Parser};
use combine::parser::char::{char};

fn make_char(ch: char) -> ByteVectorFSA {
    let mut fsa = ByteVectorFSA::new();
    let bytelen = ch.len_utf8();
    let init = fsa.add_new_state();
    let mut bytes = Vec::with_capacity(bytelen);
    bytes.resize(bytelen, 0);
    ch.encode_utf8(&mut bytes);
    let mut prev = init;
    for off in 0..bytelen {
        let next = fsa.add_new_state();
        fsa.add_arc(&prev, SimpleArc::new(bytes[off], boolweight::one(), next));
        prev = next;
    }
    fsa.set_final_weight(&prev, boolweight::one());
    fsa
}

fn make_utf8_dot() -> ByteVectorFSA {
    let mut fsa = ByteVectorFSA::new();
    let one = boolweight::one();
    let init = fsa.add_new_state();
    let finalst = fsa.add_new_state();
    fsa.set_final_weight(&finalst, boolweight::one());

    // 1 byte
    for printable_ascii in 0x20..0x7F {
        fsa.add_arc(&init, SimpleArc::new(printable_ascii, one, finalst));
    }

    // TO DO: multi-byte is not implemented yet

    fsa
}


fn make_charset(exprs: &Vec<CharSetExpr>) -> ByteVectorFSA {
    let mut ret: Option<ByteVectorFSA> = None;
    for chcode in exprs.iter().flat_map(|ex| {
        let itbox : Box<Iterator<Item=u32>> = match ex {
            &CharSetExpr::Range(beg, end) => {
                if end < beg {
                    box ((end as u32)..=(beg as u32)) // should fail?
                } else {
                    box ((beg as u32)..=(end as u32))
                }
            }
            &CharSetExpr::Char(ch) => {
                box once(ch as u32)
            }
        };
        itbox
    }) {
        let ch = char::try_from(chcode).expect("Code point refers invalid char");
        if ret.is_none() {
            ret = Some(make_char(ch));
        } else {
            ret = Some(union(ret.unwrap(), make_char(ch)));
        }
    }

    rmeps(ret.expect("Charset must have at least 1 char"))
}

fn make_charset_inv(exprs: &Vec<CharSetExpr>) -> ByteVectorFSA {
    // TO DO: Currenytly only support ASCII
    let mut ret: Option<ByteVectorFSA> = None;
    for chcode in 0x20..0x7F {
        for ex in exprs {
            let matched = match ex {
                &CharSetExpr::Range(beg, end) => {
                    (end < beg && (end as u32) <= chcode && chcode <= (beg as u32) ||
                     beg <= end && (beg as u32) <= chcode && chcode <= (end as u32))
                }
                &CharSetExpr::Char(ch) => {
                    chcode == (ch as u32)
                }
            };
            if ! matched {
                let ch = char::try_from(chcode).expect("Code point refers invalid char");
                if ret.is_none() {
                    ret = Some(make_char(ch));
                } else {
                    ret = Some(union(ret.unwrap(), make_char(ch)));
                }
            }
        }
    }

    ret.expect("Charset must have at least 1 char")
}


fn make_empty() -> ByteVectorFSA {
    let mut fsa = ByteVectorFSA::new();
    let init = fsa.add_new_state();
    fsa.set_final_weight(&init, boolweight::one());
    fsa
}

#[derive(Clone,Debug,PartialEq)]
enum CharSetExpr {
    Range(char, char),
    Char(char)
}

#[derive(Clone,Debug,PartialEq)]
enum RepeatSpec {
    Option,
    Many1,
    Many0,
    Repeat(usize, usize)
}

#[derive(Clone,Debug,PartialEq)]
enum Rexp {
    Char(char),
    CharSet(Vec<CharSetExpr>),
    CharSetInv(Vec<CharSetExpr>),
    Dot,
    Group(Box<Rexp>),
    //BOS,
    //EOS,
    Many1(Box<Rexp>),
    Many0(Box<Rexp>),
    Option(Box<Rexp>),
    Repeat(usize, usize, Box<Rexp>),
    Or(Vec<Box<Rexp>>),
    Seq(Vec<Box<Rexp>>)
}

parser! {
    fn charset_tail[I]()(I) -> Vec<CharSetExpr> where [I: Stream<Item=char>] {
        let chars = none_of("]".chars()).map(|c| { CharSetExpr::Char(c) });
        let range = (none_of("]".chars()), char('-'), none_of("]".chars())).map(|t| {
            CharSetExpr::Range(t.0, t.2)
        });
        many1(try(range).or(chars)).map(|v: Vec<CharSetExpr>| v)
    }
}

parser! {
    fn charset_expr[I]()(I) -> Vec<CharSetExpr> where [I: Stream<Item=char>] {
        (optional(char(']')), charset_tail()).map(|(ebr, tail)| {
            match ebr {
                Some(']') => {
                    let mut ret = tail.clone();
                    ret.push(CharSetExpr::Char(']'));
                    ret
                }
                Some(_) => {
                    panic!("should not reach here")
                }
                None => {
                    tail
                }
            }
        })
    }
}

parser! {
    fn rexp_repeatable[I]()(I) -> Rexp where [I: Stream<Item=char>] {
        let escaped_char = (char('\\'), one_of(".\\*+?^$()[]|".chars())).map(|x| x.1);
        let lit_char = escaped_char.or(none_of(".\\*+?^$()[]|".chars())).map(|x| Rexp::Char(x));
        let group = (char('('), rexp(), char(')')).map(|x| Rexp::Group(box x.1));
        let charset = (char('['), charset_expr(), char(']')).map(
            |t| Rexp::CharSet(t.1)
        );
        let charset_inv = (char('['), char('^'), charset_expr(), char(']')).map(
            |t| Rexp::CharSetInv(t.2)
        );
        try(group)
            .or(try(charset))
            .or(try(charset_inv))
            .or(char('.').map(|_| Rexp::Dot))
            .or(lit_char)
    }
}

parser! {
    fn rexp[I]()(I) -> Rexp where [I: Stream<Item=char>] {
        let digits = || { one_of("0123456789".chars()) };
        let num = || {
            many1(digits()).map(|s: String| s.parse::<usize>().expect("Number parse error"))
        };

        let repeat_spec = || {
            one_of("?*+".chars()).map(|ch| {
                match ch {
                    '?' => RepeatSpec::Option,
                    '*' => RepeatSpec::Many0,
                    '+' => RepeatSpec::Many1,
                    _ => panic!("Unknown repetition specifier"),
                }
            }).or(try((char('{'), num(), char('}')).map(|t| {
                RepeatSpec::Repeat(t.1, t.1)
            }))).or((char('{'), num(), char(','), num(), char('}')).map(|t| {
                RepeatSpec::Repeat(t.1, t.3)
            }))
        };

        let repeat = (rexp_repeatable::<I>(), optional(repeat_spec())).map(|(at, rpm)| {
            match rpm {
                Some(RepeatSpec::Option) => Rexp::Option(box at),
                Some(RepeatSpec::Many0) => Rexp::Many0(box at),
                Some(RepeatSpec::Many1) => Rexp::Many1(box at),
                Some(RepeatSpec::Repeat(b, e)) => Rexp::Repeat(b, e, box at),
                None => at
            }
        });

        let or_comp = many1(repeat).map(|v: Vec<Rexp>| {
            if v.len() == 1 {
                v[0].clone()
            } else {
                Rexp::Seq(v.into_iter().map(|x| box x).collect())
            }
        });

        sep_by(or_comp, char('|')).map(|v: Vec<Rexp>| {
            if v.len() == 1 {
                v[0].clone()
            } else {
                Rexp::Or(v.into_iter().map(|x| box x).collect())
            }
        })
    }
}

pub fn compile_rexp_nfa(rexp_src: &str) -> ByteVectorFSA {
    let ast = match rexp().parse(rexp_src) {
        Ok((ast, _rest)) => {
            ast
        }
        Err(what) => {
            panic!("Parse Error: {:?}", what);
        }
    };
    ast_to_fsa(&ast)
}

fn ast_to_fsa(ast: &Rexp) -> ByteVectorFSA {
    match ast {
        &Rexp::Char(ch) => make_char(ch),
        &Rexp::CharSet(ref chars) => make_charset(chars),
        &Rexp::CharSetInv(ref chars) => make_charset_inv(chars),
        &Rexp::Dot => make_utf8_dot(),
        &Rexp::Group(ref child) => ast_to_fsa(&child),
        &Rexp::Many1(ref child) => closure_plus(ast_to_fsa(&child)),
        &Rexp::Many0(ref child) => closure_star(ast_to_fsa(&child)),
        &Rexp::Repeat(b, e, ref child) => {
            let mut ret = make_empty();
            for _ in 0..b {
                ret = concat(ret, ast_to_fsa(&child));
            }
            for _ in b..e {
                ret = concat(ret, union(ast_to_fsa(&child), make_empty(), ));
            }
            ret
        }
        &Rexp::Option(ref child) => {
            union(ast_to_fsa(&child), make_empty())
        }
        &Rexp::Or(ref children) => {
            if children.len() == 0 || children.len() == 1 {
                panic!("Or-statement must have at least 2 children")
            } else {
                let mut ret = concat(ast_to_fsa(&children[0]), ast_to_fsa(&children[1]));
                for i in 2..(children.len()) {
                    ret = union(ret, ast_to_fsa(&children[i]));
                }
                ret
            }

        }
        &Rexp::Seq(ref children) => {
            if children.len() == 0 {
                make_empty()
            } else if children.len() == 1 {
                ast_to_fsa(children.first().unwrap())
            } else {
                let mut ret = concat(ast_to_fsa(&children[0]), ast_to_fsa(&children[1]));
                for i in 2..(children.len()) {
                    ret = concat(ret, ast_to_fsa(&children[i]));
                }
                ret
            }
        }
    }
}

#[test]
pub fn rexp_parser_test() {
    use automata::DumpTSV;

    use self::Rexp::{Seq,Option,Group,Char,Dot,CharSet,Many0,Many1,Or};
    use self::CharSetExpr::{Range};
    use automata::{Arc};
    use automata::arcsort::arcsort;
    use automata::determinize::determinize;

    let rexp_src = r#"(a\.)?(abc)+.*[A-Z]+|abc"#;
    let result = rexp().parse(rexp_src);

    let ast = match result {
        Ok((ast, rest)) => {
            let expected =
                Or(vec!(
                    box Seq(vec!(box Option(box Group(box Seq(vec!(box Char('a'), box Char('.'))))),
                             box Many1(box Group(box Seq(vec!(box Char('a'), box Char('b'), box Char('c'))))),
                             box Many0(box Dot),
                             box Many1(box CharSet(vec!(Range('A', 'Z')))))
                    ),
                    box Seq(vec!(box Char('a'), box Char('b'), box Char('c')))
                ));
            assert!(rest == "");
            assert!(ast == expected);
            ast
        }
        Err(what) => {
            println!("Parse Error: {:?}", what);
            panic!("parse error")
        }
    };
    let fsa = arcsort(determinize(rmeps(ast_to_fsa(&ast))),
                      |ref x, ref y| { x.label().cmp(&y.label()) } );

    let mut dump_buf = Vec::<u8>::new();
    fsa.dump_tsv(&mut dump_buf);
    let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");

    println!("Rexp Parse Result[{}]\n====\n{}", rexp_src, dumped);
}

#[test]
pub fn compile_rexp_test() {
    use automata::{DumpTSV};

    {
        let expected = "
0	1	97	true
1	2	98	true
2	3	99	true
3	true
".trim();
        let fsa = rmeps(compile_rexp_nfa("abc"));

        let mut dump_buf = Vec::<u8>::new();
        fsa.dump_tsv(&mut dump_buf);
        let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");

        assert!(dumped.trim() == expected);
    }

    {
        let expected = "
0	1	97	true
1	2	97	true
2	3	97	true
3	true
".trim();
        let fsa = rmeps(compile_rexp_nfa("a{3}"));

        let mut dump_buf = Vec::<u8>::new();
        fsa.dump_tsv(&mut dump_buf);
        let dumped = String::from_utf8(dump_buf).expect("UTF-8 error");

        assert_eq!(dumped.trim(), expected);
    }

}
