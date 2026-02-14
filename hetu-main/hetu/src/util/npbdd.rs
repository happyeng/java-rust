use crate::util::rule::Rule;
use biodivine_lib_bdd::*;
use dashmap::DashMap;
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct NPBDD;
impl NPBDD {
    pub fn init(ip_bits_len: usize) {
        Engine::init(ip_bits_len);
        BDDTable::init();
        LayerCache::init();
    }

    pub fn make_none_space_bdd() -> Bdd {
        Engine::make_none_space_bdd()
    }

    pub fn make_all_space_bdd() -> Bdd {
        Engine::make_all_space_bdd()
    }

    pub fn make_prefix_bdd(ip_address: &str, prefix_length: usize) -> Bdd {
        Engine::encode_dst_ip_prefix_clause(ip_address, prefix_length)
    }

    pub fn make_src_device_bdd(src_device_id: usize) -> Bdd {
        Engine::encode_src_device_constraint(src_device_id)
    }
}

// Canonical representation of symbolic structures
lazy_static! {
    static ref NUM_TO_BDD_MAP: DashMap<u32, Bdd> = DashMap::new();
    static ref BDD_TO_NUM_MAP: DashMap<Bdd, u32> = DashMap::new();
    static ref BDD_TABLE_NEXT_ID: AtomicUsize = AtomicUsize::new(1);
    static ref PREFIX_BDD_MAP: DashMap<String, u32> = DashMap::new();
}

pub struct BDDTable;
impl BDDTable {
    pub fn init() {}

    pub fn get_next_id() -> usize {
        BDD_TABLE_NEXT_ID.fetch_add(1, Ordering::SeqCst)
    }

    pub fn get_id_num() -> usize {
        NUM_TO_BDD_MAP.len()
    }

    pub fn get_bdd_num() -> usize {
        BDD_TO_NUM_MAP.len()
    }

    pub fn insert_bdd(bdd: Bdd) -> u32 {
        if let Some(existing_id) = BDD_TO_NUM_MAP.get(&bdd) {
            return *existing_id;
        }
        let new_id = BDD_TABLE_NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let id_u32 = new_id.try_into().unwrap();
        NUM_TO_BDD_MAP.insert(id_u32, bdd.clone());
        BDD_TO_NUM_MAP.insert(bdd, id_u32);
        id_u32
    }

    pub fn get_bdd_by_id(id: u32) -> Option<Bdd> {
        NUM_TO_BDD_MAP.get(&id).map(|bdd| bdd.clone())
    }

    pub fn get_prefix_bdd_map() -> &'static DashMap<String, u32> {
        &PREFIX_BDD_MAP
    }
}

// Hierarchical memoization across computation granularities
lazy_static! {
    static ref L3_MAKE_CACHE: DashMap<String, u32> = DashMap::new();
    static ref L3_AND_CACHE: DashMap<(u32, u32), u32> = DashMap::new();
    static ref L3_OR_CACHE: DashMap<(u32, u32), u32> = DashMap::new();
    static ref L3_NOT_CACHE: DashMap<u32, u32> = DashMap::new();
    static ref L2_ENCODE_RULE_CACHE: DashMap<(String, usize), u32> = DashMap::new();
    static ref L2_CAL_HIT_CACHE: DashMap<(u32, u32), (u32, u32)> = DashMap::new();
    // Key/value MUST preserve the caller-provided order; never sort or hash port lists.
    static ref L2_MERGE_PORT_SPACE_CACHE: DashMap<(u32, Vec<u32>), Vec<u32>> = DashMap::new();
    static ref L1_COMPLETE_RULE_CACHE: DashMap<(String, u32, Vec<u32>), (u32, Vec<u32>)> =
        DashMap::new();
    static ref L1_HIT_CNT: AtomicUsize = AtomicUsize::new(0);
    static ref L2_HIT_CNT: AtomicUsize = AtomicUsize::new(0);
    static ref L3_HIT_CNT: AtomicUsize = AtomicUsize::new(0);
    static ref MISS_CNT: AtomicUsize = AtomicUsize::new(0);
}

pub struct LayerCache;
impl LayerCache {
    pub fn init() {}

    // Atomic operations preserved in memory
    pub fn l3_make(ip: &str, prefix_len: usize) -> u32 {
        let key = format!("{}/{}", ip, prefix_len);

        if let Some(cached_id) = L3_MAKE_CACHE.get(&key) {
            L3_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            return *cached_id;
        }

        MISS_CNT.fetch_add(1, Ordering::Relaxed);
        let bdd = NPBDD::make_prefix_bdd(ip, prefix_len);
        let bdd_id = BDDTable::insert_bdd(bdd);
        L3_MAKE_CACHE.insert(key, bdd_id);
        bdd_id
    }

    pub fn l3_and(a_id: u32, b_id: u32) -> u32 {
        let key = (a_id, b_id);

        if let Some(cached_id) = L3_AND_CACHE.get(&key) {
            L3_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            return *cached_id;
        }

        MISS_CNT.fetch_add(1, Ordering::Relaxed);
        let a = BDDTable::get_bdd_by_id(a_id).unwrap();
        let b = BDDTable::get_bdd_by_id(b_id).unwrap();
        let result_bdd = a.and(&b);
        let result_id = BDDTable::insert_bdd(result_bdd);
        L3_AND_CACHE.insert(key, result_id);
        result_id
    }

    pub fn l3_or(a_id: u32, b_id: u32) -> u32 {
        let key = (a_id, b_id);

        if let Some(cached_id) = L3_OR_CACHE.get(&key) {
            L3_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            return *cached_id;
        }

        MISS_CNT.fetch_add(1, Ordering::Relaxed);
        let a = BDDTable::get_bdd_by_id(a_id).unwrap();
        let b = BDDTable::get_bdd_by_id(b_id).unwrap();
        let result_bdd = a.or(&b);
        let result_id = BDDTable::insert_bdd(result_bdd);
        L3_OR_CACHE.insert(key, result_id);
        result_id
    }

    pub fn l3_not(id: u32) -> u32 {
        if let Some(cached_id) = L3_NOT_CACHE.get(&id) {
            L3_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            return *cached_id;
        }

        MISS_CNT.fetch_add(1, Ordering::Relaxed);
        let bdd = BDDTable::get_bdd_by_id(id).unwrap();
        let result_bdd = bdd.not();
        let result_id = BDDTable::insert_bdd(result_bdd);
        L3_NOT_CACHE.insert(id, result_id);
        result_id
    }

    // Compositional patterns captured in intermediate layers
    pub fn l2_encode_rule(ip: &str, prefix_len: usize) -> u32 {
        let key = (ip.to_string(), prefix_len);

        if let Some(cached_id) = L2_ENCODE_RULE_CACHE.get(&key) {
            L2_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            return *cached_id;
        }

        let prefix_bdd_id = Self::l3_make(ip, prefix_len);
        L2_ENCODE_RULE_CACHE.insert(key, prefix_bdd_id);
        prefix_bdd_id
    }

    pub fn l2_cal_hit(prefix_bdd_id: u32, used_space_id: u32) -> (u32, u32) {
        let key = (prefix_bdd_id, used_space_id);

        if let Some(cached_result) = L2_CAL_HIT_CACHE.get(&key) {
            L2_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            return *cached_result.value();
        }

        let not_used_space_id = Self::l3_not(used_space_id);
        let hit_id = Self::l3_and(prefix_bdd_id, not_used_space_id);
        let new_used_space_id = Self::l3_or(used_space_id, hit_id);

        let result = (hit_id, new_used_space_id);
        L2_CAL_HIT_CACHE.insert(key, result);
        result
    }

    pub fn l2_merge_port_space(hit_id: u32, old_port_space_ids: &[u32]) -> Vec<u32> {
        // IMPORTANT: order is semantically meaningful (caller zips results back to ports).
        let key = (hit_id, old_port_space_ids.to_vec());

        if let Some(cached_result) = L2_MERGE_PORT_SPACE_CACHE.get(&key) {
            L2_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            return cached_result.value().clone();
        }

        let new_port_space_ids: Vec<u32> = old_port_space_ids
            .iter()
            .map(|&old_port_id| Self::l3_or(old_port_id, hit_id))
            .collect();

        L2_MERGE_PORT_SPACE_CACHE.insert(key, new_port_space_ids.clone());
        new_port_space_ids
    }

    // Complete transformations as holistic units
    pub fn l1_complete_rule(
        rule: &Rule,
        used_space_id: u32,
        port_space_ids: &[u32],
    ) -> Option<(u32, Vec<u32>)> {
        let rule_descriptor = format!("{}/{}", rule.get_ip(), rule.get_prefix_len());
        // IMPORTANT: port_space_ids order is part of the key.
        let key = (rule_descriptor, used_space_id, port_space_ids.to_vec());

        if let Some(cached_result) = L1_COMPLETE_RULE_CACHE.get(&key) {
            L1_HIT_CNT.fetch_add(1, Ordering::Relaxed);
            let (new_used_space_id, new_port_ids) = cached_result.value();
            return Some((*new_used_space_id, new_port_ids.clone()));
        }

        None
    }

    pub fn l1_cache_result(
        rule: &Rule,
        used_space_id: u32,
        port_space_ids: &[u32],
        new_used_space_id: u32,
        new_port_space_ids: &[u32],
    ) {
        let rule_descriptor = format!("{}/{}", rule.get_ip(), rule.get_prefix_len());
        let key = (rule_descriptor, used_space_id, port_space_ids.to_vec());
        let value = (new_used_space_id, new_port_space_ids.to_vec());
        L1_COMPLETE_RULE_CACHE.insert(key, value);
    }

    // Legacy interface adaptations
    pub fn cached_or(a_id: u32, b_id: u32) -> u32 {
        Self::l3_or(a_id, b_id)
    }

    pub fn cached_prefix_match(all_bdd_id: u32, bdd_hit_id: u32) -> (u32, u32) {
        let not_all_bdd_id = Self::l3_not(all_bdd_id);
        let hit_id = Self::l3_and(bdd_hit_id, not_all_bdd_id);
        let new_all_bdd_id = Self::l3_or(all_bdd_id, hit_id);
        (new_all_bdd_id, hit_id)
    }

    pub fn cached_relevance(all_space_id: u32, bdd_match_id: u32) -> bool {
        let intersection_id = Self::l3_and(all_space_id, bdd_match_id);
        let intersection_bdd = BDDTable::get_bdd_by_id(intersection_id).unwrap();
        !intersection_bdd.is_false()
    }

    pub fn get_cache_stats() -> (usize, usize, usize, usize) {
        (
            L1_HIT_CNT.load(Ordering::Relaxed),
            L2_HIT_CNT.load(Ordering::Relaxed),
            L3_HIT_CNT.load(Ordering::Relaxed),
            MISS_CNT.load(Ordering::Relaxed),
        )
    }
}

// Symbolic encoding of network semantics
struct Engine;
static IP_BITS_LEN: OnceCell<usize> = OnceCell::new();
static IP_BIT_VARIABLES: OnceCell<Vec<BddVariable>> = OnceCell::new();
static VARIABLE_SET: OnceCell<BddVariableSet> = OnceCell::new();
impl Engine {
    fn init(ip_bits_len: usize) {
        let _ = IP_BITS_LEN.set(ip_bits_len);
        let mut variable_builder = BddVariableSetBuilder::new();
        let mut ip_bit_variables = Vec::new();

        for i in 0..ip_bits_len {
            let var_name = format!("x{}", i + 1);
            let var = variable_builder.make_variable(&var_name);
            ip_bit_variables.push(var);
        }
        let variable_set = variable_builder.build();
        let _ = IP_BIT_VARIABLES.set(ip_bit_variables);
        let _ = VARIABLE_SET.set(variable_set);
    }

    fn ip_bits_len() -> usize {
        *IP_BITS_LEN.get().expect("IP_BITS_LEN not initialized")
    }

    fn ip_bit_variables() -> &'static [BddVariable] {
        IP_BIT_VARIABLES
            .get()
            .expect("IP_BIT_VARIABLES not initialized")
    }

    fn variable_set() -> &'static BddVariableSet {
        VARIABLE_SET.get().expect("VARIABLE_SET not initialized")
    }

    fn make_none_space_bdd() -> Bdd {
        VARIABLE_SET
            .get()
            .expect("VARIABLE_SET not initialized")
            .mk_false()
    }

    fn make_all_space_bdd() -> Bdd {
        VARIABLE_SET
            .get()
            .expect("VARIABLE_SET not initialized")
            .mk_true()
    }

    fn encode_dst_ip_prefix_clause(ip_address: &str, prefix_length: usize) -> Bdd {
        let _ip_bits_len = Self::ip_bits_len();
        let variables = Self::ip_bit_variables();
        let variable_set = Self::variable_set();

        if prefix_length == 0 {
            return variable_set.mk_true();
        }

        let ip_addr: IpAddr = ip_address.parse().unwrap();
        let ip_bits: Vec<bool> = match ip_addr {
            IpAddr::V4(ipv4) => ipv4
                .octets()
                .iter()
                .flat_map(|&octet| (0..8).rev().map(move |i| (octet & (1 << i)) != 0))
                .collect(),
            IpAddr::V6(ipv6) => ipv6
                .octets()
                .iter()
                .flat_map(|&octet| (0..8).rev().map(move |i| (octet & (1 << i)) != 0))
                .collect(),
        };

        let mut values = Vec::new();
        let range_variables = &variables[(144 - prefix_length)..144];
        for (i, &var) in range_variables.iter().rev().enumerate() {
            values.push((var, ip_bits[i]));
        }

        let clause = BddPartialValuation::from_values(&values);
        variable_set.mk_conjunctive_clause(&clause)
    }

    fn encode_src_device_constraint(src_device_id: usize) -> Bdd {
        let variables = Self::ip_bit_variables();
        let variable_set = Self::variable_set();

        let device_bits: Vec<bool> = (0..16)
            .rev()
            .map(|i| (src_device_id & (1 << i)) != 0)
            .collect();

        let mut values = Vec::new();
        let range_variables = &variables[0..16];
        for (i, &var) in range_variables.iter().rev().enumerate() {
            values.push((var, device_bits[i]));
        }

        let clause = BddPartialValuation::from_values(&values);
        variable_set.mk_conjunctive_clause(&clause)
    }
}
