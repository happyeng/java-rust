use biodivine_lib_bdd::Bdd;
use std::hash::{Hash, Hasher};

use crate::util::forward_action::ForwardAction;

#[derive(Clone)]
pub struct Lec {
    pub forward_action: ForwardAction,
    pub predicate: Bdd,
}

impl Lec {
    pub fn new(forward_action: ForwardAction, predicate: Bdd) -> Self {
        Lec {
            forward_action,
            predicate,
        }
    }
}

impl Hash for Lec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.forward_action.hash(state);
        self.predicate.hash(state);
    }
}

impl PartialEq for Lec {
    fn eq(&self, other: &Self) -> bool {
        self.forward_action == other.forward_action && self.predicate == other.predicate
    }
}

impl Eq for Lec {}
