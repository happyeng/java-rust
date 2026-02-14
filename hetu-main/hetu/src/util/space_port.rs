use biodivine_lib_bdd::Bdd;
use hashbrown::HashSet;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct SpacePort {
    space_id: i8,
    space: Bdd,
    cache_table: HashSet<Bdd>,
}

impl Hash for SpacePort {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.space_id.hash(state);
        self.space.hash(state);
    }
}

impl PartialEq for SpacePort {
    fn eq(&self, other: &Self) -> bool {
        self.space_id == other.space_id && self.space == other.space
    }
}

impl Eq for SpacePort {}

impl SpacePort {
    pub fn new(space_id: i8, space: Bdd) -> SpacePort {
        SpacePort {
            space_id,
            space,
            cache_table: HashSet::default(),
        }
    }

    pub fn get_space_id(&self) -> i8 {
        self.space_id
    }

    pub fn get_space(&self) -> &Bdd {
        &self.space
    }

    pub fn check_cache_space(&self, cache_space: &Bdd) -> bool {
        self.cache_table.contains(cache_space)
    }

    pub fn insert_cache_space(&mut self, cache_space: &Bdd) {
        self.cache_table.insert(cache_space.clone());
    }
}
