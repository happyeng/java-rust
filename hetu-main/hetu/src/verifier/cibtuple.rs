use biodivine_lib_bdd::Bdd;
#[derive(Clone)]
pub struct CibTuple {
    predicate: Bdd, //
    count: i32,
}

impl CibTuple {
    pub fn new(predicate: Bdd, count: i32) -> Self {
        CibTuple { predicate, count }
    }

    pub fn keep_and_split(&mut self, pre: Bdd, count: i32) -> CibTuple {
        let new_pre = self.predicate.and(&pre);
        let not_new_pre = self.predicate.and_not(&pre);
        self.predicate = new_pre;
        self.count = count;
        CibTuple::new(not_new_pre, 0)
    }

    pub fn set_count(&mut self, count: i32) {
        self.count = count;
    }

    pub fn get_count(&self) -> i32 {
        self.count
    }

    pub fn set_predicate(&mut self, new_predicate: Bdd) {
        self.predicate = new_predicate
    }

    pub fn get_predicate(&self) -> &Bdd {
        &self.predicate
    }
}
