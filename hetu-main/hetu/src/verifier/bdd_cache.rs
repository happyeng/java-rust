use biodivine_lib_bdd::Bdd;
use hashbrown::{HashMap, HashSet};

#[derive(Clone)]
pub struct BddCache {
    last_bdd_pair_and: Option<(Bdd, Bdd)>,
    last_bdd_pair_or: Option<(Bdd, Bdd)>,
    arrive_predicate: Bdd,
    predicate_type_set: HashSet<Bdd>,
    and_table: HashMap<Bdd, Bdd>,
}

impl BddCache {
    pub fn new(arrive_predicate: Bdd) -> Self {
        BddCache {
            last_bdd_pair_and: None,
            last_bdd_pair_or: None,
            arrive_predicate: arrive_predicate,
            predicate_type_set: HashSet::default(),
            and_table: HashMap::default(),
        }
    }

    pub fn get_type_len(&self) -> usize {
        self.predicate_type_set.len()
    }

    pub fn get_intersection(&mut self, predicate: &Bdd) -> Bdd {
        if let Some((last_pred, last_result_bdd)) = &self.last_bdd_pair_and {
            if last_pred == predicate {
                return last_result_bdd.clone();
            }
        }
        self.predicate_type_set.insert(predicate.clone());
        let intersection = self.arrive_predicate.and(predicate);
        self.last_bdd_pair_and = Some((predicate.clone(), intersection.clone()));
        intersection
    }

    pub fn get_intersection_with_table(&mut self, predicate: &Bdd) -> Bdd {
        if let Some(result) = self.and_table.get(predicate) {
            return result.clone();
        }

        let intersection = self.arrive_predicate.and(predicate);

        self.and_table
            .insert(predicate.clone(), intersection.clone());

        intersection
    }

    pub fn get_union(&mut self, predicate: &Bdd) -> Bdd {
        if let Some((last_pred, last_result_bdd)) = &self.last_bdd_pair_or {
            if last_pred == predicate {
                return last_result_bdd.clone();
            }
        }
        let union = self.arrive_predicate.or(predicate);
        self.last_bdd_pair_or = Some((predicate.clone(), union.clone()));
        union
    }
}
