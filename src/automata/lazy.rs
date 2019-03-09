use automata::{State,Arc};

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Utility class for supporting lazy expansion of state machines
pub struct ArcCache<S: State, A: Arc<State=S>> {
    data: RefCell<BTreeMap<S, Rc<Vec<A>>>>
}

impl<S: State, A: Arc<State=S>> ArcCache<S, A> {
    pub fn new() -> ArcCache<S, A> {
        ArcCache {
            data: RefCell::new(BTreeMap::new())
        }
    }

    pub fn query<'a, 'b, F>(&'a self, s: &'b S, exp: &'b F) -> Box<'a + Iterator<Item=A>>
        where F: Fn(&'b S,) -> Box<'a + Iterator<Item=A>>{
        let cached = self.data.borrow().get(&s).map(|x| x.clone());

        match cached {
            Some(vec) => {
                box vec.to_vec().into_iter()
            }
            None => {
                let mut v = Vec::new();
                for arc in exp(s) {
                    v.push(arc);
                }
                self.data.borrow_mut().insert(s.clone(), Rc::new(v));

                let added = self.data.borrow().get(s).cloned().unwrap();
                box added.to_vec().into_iter()
            }
        }
    }
}
