use biodivine_lib_bdd::*;

#[derive(Clone)]
pub struct RuleBDD {
    hit: Bdd,
    tmatch: Bdd,
    lec_index: i32,
    black_list: Vec<Bdd>,
}

impl RuleBDD {
    pub fn new(hit: Bdd, tmatch: Bdd, lec_index: i32) -> Self {
        RuleBDD {
            hit: hit,
            tmatch: tmatch,
            lec_index: lec_index,
            black_list: Vec::new(),
        }
    }

    pub fn compare_with_other_rule_bdd(&self, o_rule_bdd: &RuleBDD) -> bool {
        let result = self.hit.xor(o_rule_bdd.get_hit());
        result.is_false()
    }

    pub fn get_hit(&self) -> &Bdd {
        &self.hit
    }

    pub fn get_match(&self) -> &Bdd {
        &self.tmatch
    }

    pub fn get_blacklist(&self) -> &Vec<Bdd> {
        &self.black_list
    }

    pub fn set_hit(&mut self, bdd: Bdd) {
        self.hit = bdd;
    }

    pub fn add_blacklist(&mut self, bdd: Bdd) {
        self.black_list.push(bdd);
    }
}
