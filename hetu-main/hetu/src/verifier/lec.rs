use crate::util::forward_action::ForwardAction;
use biodivine_lib_bdd::Bdd;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

#[derive(Clone)]
pub struct Lec {
    pub forward_action: ForwardAction,
    pub predicate: Bdd,
    exhausted: OnceLock<bool>,
}

impl Lec {
    pub fn new(forward_action: ForwardAction, predicate: Bdd) -> Self {
        Lec {
            forward_action,
            predicate,
            exhausted: OnceLock::new(),
        }
    }

    pub fn set_exhausted(&self) {
        self.exhausted.get_or_init(|| true);
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted.get().is_some()
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
