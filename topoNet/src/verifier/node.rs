use crate::util::network::Network;
use crate::verifier::annoucement::Announcement;
use crate::verifier::cibtuple::CibTuple;
use crate::verifier::context::Ctx;
use crate::verifier::lec::Lec;
use crate::{EXIST_COUNT, NONEXIST_COUNT};
use biodivine_lib_bdd::Bdd;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;

#[derive(Clone)]
pub struct Node {
    name: String,
    in_degree_lec_cnt: i32,
    local_cib: HashMap<String, CibTuple>,
    port_cib: HashMap<String, Vec<CibTuple>>,
}

impl Node {
    pub fn new(name: String) -> Self {
        Node {
            name,
            in_degree_lec_cnt: 0,
            local_cib: HashMap::new(),
            port_cib: HashMap::new(),
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_indegree_lec_cnt(&self) -> i32 {
        self.in_degree_lec_cnt.clone()
    }

    pub fn init_cib(
        &mut self,
        network: &Network,
        packet_space_bdd: Bdd,
        lecs: &HashSet<Lec>,
        _dst_node_name: String,
    ) {
        if let Some(adj_ports) = network.get_device_ports().get(&self.name.clone()) {
            for adj_port in adj_ports {
                self.port_cib
                    .insert(adj_port.get_port_name().to_string(), Vec::new());
            }
        }
        for lec in lecs.clone() {
            let intersection_bdd = packet_space_bdd.and(&lec.predicate);
            if intersection_bdd.is_false() {
                continue;
            } else {
                let new_cibtuple =
                    CibTuple::new(intersection_bdd.clone(), lec.forward_action.clone(), 0);
                for port in lec.forward_action.get_ports() {
                    if let Some(vec) = self.port_cib.get_mut(port) {
                        vec.push(new_cibtuple.clone());
                    }
                    self.local_cib.insert(port.clone(), new_cibtuple.clone());
                }
                self.in_degree_lec_cnt += 1;
            }
        }
    }

    pub fn update_loc_cib(&mut self, from_port_name: String, annoucement: Announcement) -> bool {
        let cibtuples = self.port_cib.get_mut(&from_port_name).unwrap();
        if let Some(mut cibtuple) = cibtuples.pop() {
            let intersection_bdd = annoucement.get_predicate().and(cibtuple.get_predicate());
            if intersection_bdd != cibtuple.get_predicate().clone() {
                let old_cibtuple = self.local_cib.get_mut(&from_port_name).unwrap();
                old_cibtuple.set_predicate(intersection_bdd.clone());
                old_cibtuple.set_count(1);
                let new_cibtuple = cibtuple.keep_and_split(intersection_bdd, 1);
                cibtuples.push(new_cibtuple);
                return false;
            } else {
                let old_cibtuple = self.local_cib.get_mut(&from_port_name).unwrap();
                old_cibtuple.set_predicate(intersection_bdd.clone());
                old_cibtuple.set_count(1);
                self.in_degree_lec_cnt -= 1;
                return true;
            }
        }
        false
    }

    pub fn count_check(
        &mut self,
        port_name: String,
        current_ctx: &Ctx,
        edge_devices: &HashSet<String>,
    ) -> bool {
        let annoucement = current_ctx.get_announcement();
        if self.local_cib.is_empty() {
            return false;
        }
        if !self.update_loc_cib(port_name.clone(), annoucement) {
            return false;
        }
        if edge_devices.contains(&self.get_name()) {
            return false;
        }
        if self.in_degree_lec_cnt != 0 {
            return false;
        }
        true
    }

    pub fn get_cib_out(&mut self) -> Announcement {
        let mut has_result = false;
        let mut count_predicate: HashMap<i32, Bdd> = HashMap::new();
        for cibtuple in self.local_cib.values() {
            let tmp_count = cibtuple.get_count();
            if tmp_count == 1 {
                has_result = true;
            }
            let tmp_bdd = cibtuple.get_predicate();
            if !count_predicate.contains_key(&tmp_count) {
                count_predicate.insert(tmp_count, tmp_bdd.clone());
            } else {
                let new_bdd = count_predicate.get(&tmp_count).unwrap().clone().or(tmp_bdd);
                count_predicate.insert(tmp_count, new_bdd);
            }
        }
        if !has_result {
            let annoucement_out: Announcement =
                Announcement::new(count_predicate.get(&0).unwrap().clone(), 0);
            annoucement_out
        } else {
            let annoucement_out = Announcement::new(count_predicate.get(&1).unwrap().clone(), 1);
            annoucement_out
        }
    }

    pub fn get_result(&mut self, dst_packet_space_bdd: &Bdd) -> bool {
        if self.local_cib.len() == 0 {
            NONEXIST_COUNT.fetch_add(1, Ordering::SeqCst);
            false
        } else {
            let annoucement_final = self.get_cib_out();
            let packet_space_bdd_final = annoucement_final.get_predicate();
            let count_final = annoucement_final.get_count();
            if count_final == 1 && packet_space_bdd_final == dst_packet_space_bdd {
                EXIST_COUNT.fetch_add(1, Ordering::SeqCst);
                true
            } else {
                NONEXIST_COUNT.fetch_add(1, Ordering::SeqCst);
                false
            }
        }
    }
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Node {}
