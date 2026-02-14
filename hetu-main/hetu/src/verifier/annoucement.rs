use biodivine_lib_bdd::Bdd;
#[derive(Clone)]
pub struct Announcement {
    predicate: Bdd,
    count: i32,
}

impl Announcement {
    pub fn new(predicate: Bdd, count: i32) -> Self {
        Announcement { predicate, count }
    }

    pub fn get_predicate(&self) -> &Bdd {
        &self.predicate
    }

    pub fn get_count(&self) -> i32 {
        self.count
    }
}
