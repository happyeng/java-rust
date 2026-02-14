use super::annoucement::Announcement;
use super::bdd_cache::BddCache;
use super::context::Ctx;
use super::device::Device;
use super::device::ALIVE_DEVICE_MAP;
use super::lec::Lec;
use super::space_node::SpaceNode;
use crate::util::hash_utils::{HashMap, HashSet};
use crate::util::network::Network;
use crate::util::space_port::SpacePort;
use crate::verifier::cibtuple::CibTuple;
use biodivine_lib_bdd::Bdd;
use std::sync::Arc;

#[derive(Clone)]
pub struct Node {
    pub name: Arc<String>,
    port_name_to_space_id: HashMap<String, i8>,
    space_id_to_space_port: HashMap<i8, SpacePort>,
    space_id_to_conceptual_space: HashMap<i8, Bdd>,
    local_cib: HashMap<String, CibTuple>,
    port_cib: HashMap<String, Vec<CibTuple>>,
    locked_space_port: HashSet<i8>,
    device: Option<Arc<Device>>,
    pub port_arrive_cnt: HashMap<String, i32>,
    round_space: Option<Bdd>,
    pub verify_space: Option<Bdd>,
}

impl Node {
    pub fn new(name: Arc<String>) -> Self {
        Node {
            name,
            local_cib: HashMap::default(),
            port_cib: HashMap::default(),
            port_name_to_space_id: HashMap::default(),
            space_id_to_space_port: HashMap::default(),
            space_id_to_conceptual_space: HashMap::default(),
            locked_space_port: HashSet::default(),
            device: None,
            port_arrive_cnt: HashMap::default(),
            round_space: None,
            verify_space: None,
        }
    }

    pub fn set_device(&mut self, device: Arc<Device>) {
        self.device = Some(device);
    }

    pub fn get_device(&self) -> Arc<Device> {
        self.device.as_ref().expect("Device not set").clone()
    }

    pub fn reset_node(&mut self) {
        self.port_name_to_space_id.clear();
        self.space_id_to_space_port.clear();
        self.space_id_to_conceptual_space.clear();
        self.local_cib.clear();
        self.port_cib.clear();
        self.locked_space_port.clear();
    }

    pub fn get_name(&self) -> String {
        self.name.to_string()
    }

    pub fn is_local_cib_empty(&self) -> bool {
        self.local_cib.is_empty()
    }

    pub fn init_cib_new(
        &mut self,
        network: &Network,
        packet_space_bdd: &Bdd,
        lecs: &HashSet<Lec>,
        dst_node_name: String,
    ) {
        for lec in lecs {
            let intersection_bdd = packet_space_bdd.and(&lec.predicate);
            if intersection_bdd.is_false() {
                continue;
            } else {
                let ports = lec.forward_action.get_ports();
                for port in ports {
                    let new_cibtuple = CibTuple::new(intersection_bdd.clone(), 0);
                    if let Some(vec) = self.port_cib.get_mut(port) {
                        vec.push(new_cibtuple);
                    } else {
                        let mut vec: Vec<CibTuple> = Vec::new();
                        vec.push(new_cibtuple);
                        self.port_cib.insert(port.to_string(), vec);
                    }
                }
            }
        }
    }

    pub fn cal_theoretical_space(
        &mut self,
        space_id_to_space_port: &HashMap<i8, SpacePort>,
        packet_space: &Bdd,
        tmp_bdd_cache: &mut BddCache,
    ) {
        for (space_id, space_port) in space_id_to_space_port {
            let space = space_port.get_space();
            let result = packet_space.and(space);
            if !result.is_false() {
                self.space_id_to_conceptual_space.insert(*space_id, result);
            }
        }
    }

    pub fn check_alive_device_and_space_port(&self, space_id: i8) -> bool {
        if let Some(alive_space_port_set) = ALIVE_DEVICE_MAP.get_mut(&self.name.to_string()) {
            alive_space_port_set.contains(&(space_id as i16))
        } else {
            false
        }
    }

    pub fn update_alive_device_and_space_port(&self, space_id: i8) {
        if let Some(alive_space_port_set) = ALIVE_DEVICE_MAP.get_mut(&self.name.to_string()) {
            alive_space_port_set.remove(&(space_id as i16));
        }
    }

    pub fn init_cib_space_port(&mut self, packet_space_bdd: &Bdd, lecs: &HashSet<Lec>) {
        let mut tmp_space_port: HashMap<Bdd, i8> = HashMap::default();
        let mut cur_space_id = 0;

        for lec in lecs {
            if lec.is_exhausted() {
                continue;
            }

            if *packet_space_bdd == lec.predicate {
                lec.set_exhausted();
            }

            let intersection_bdd = packet_space_bdd.and(&lec.predicate);

            if intersection_bdd.is_false() {
                continue;
            } else {
                let ports = lec.forward_action.get_ports();
                let port_name = match ports.first() {
                    Some(name) => name,
                    None => {
                        eprintln!("Ports list is empty");
                        return;
                    }
                };
                if let Some(&tmp_space_id) = tmp_space_port.get(&intersection_bdd) {
                    self.port_name_to_space_id
                        .insert(port_name.to_string(), tmp_space_id);
                } else {
                    let new_space_port = SpacePort::new(cur_space_id, intersection_bdd.clone());
                    self.space_id_to_space_port
                        .insert(cur_space_id, new_space_port);
                    self.port_name_to_space_id
                        .insert(port_name.to_string(), cur_space_id);
                    tmp_space_port.insert(intersection_bdd, cur_space_id);
                    cur_space_id += 1;
                }
            }
        }
    }

    pub fn update_loc_cib_new(
        &mut self,
        from_port_name: String,
        annoucement: Announcement,
    ) -> bool {
        let cibtuples = self.port_cib.get_mut(&from_port_name).unwrap();
        if let Some(mut cibtuple) = cibtuples.pop() {
            let intersection_bdd = annoucement.get_predicate().and(cibtuple.get_predicate());
            if intersection_bdd != cibtuple.get_predicate().clone() {
                let mut verified_cibtuple = cibtuple.clone();
                verified_cibtuple.set_predicate(intersection_bdd.clone());
                verified_cibtuple.set_count(1);
                let new_cibtuple = cibtuple.keep_and_split(intersection_bdd, 1);
                cibtuples.push(new_cibtuple);
                self.local_cib.insert(from_port_name, verified_cibtuple);
                return false;
            } else {
                cibtuple.set_count(1);
                self.local_cib.insert(from_port_name, cibtuple);
                return true;
            }
        }
        false
    }

    pub fn update_loc_cib_new_by_space_port(
        &mut self,
        from_port_name: String,
        annoucement: Announcement,
    ) -> bool {
        let arrive_predicate = annoucement.get_predicate();
        let space_id = self.port_name_to_space_id.get(&from_port_name).unwrap();
        let space_port = self
            .space_id_to_space_port
            .get_mut(space_id)
            .unwrap_or_else(|| {
                panic!(
                    "Error: the value of space_id {} is not found in space_id_to_space_port",
                    space_id
                )
            });
        if !space_port.check_cache_space(arrive_predicate) {
            let space = space_port.get_space();
            let intersection_bdd = arrive_predicate.and(space);
            let cibtuple = CibTuple::new(intersection_bdd, 1);
            self.local_cib.insert(from_port_name, cibtuple);
            space_port.insert_cache_space(arrive_predicate);
        } else {
        }
        return true;
    }

    pub fn update_loc_cib_with_forward_lock_checking(
        &mut self,
        dst_device: &Device,
        from_port_name: String,
        annoucement: Announcement,
        dst_packet_space_bdd: &Bdd,
        bdd_cache: &mut BddCache,
    ) -> bool {
        let space_id = dst_device.get_space_id(&from_port_name);
        let theoretical_space = match self.space_id_to_conceptual_space.get(&space_id) {
            Some(space) => space,
            None => {
                return false;
            }
        };

        let arrive_space = annoucement.get_predicate();
        if theoretical_space != arrive_space {
            let extra_space = theoretical_space.and_not(arrive_space);
            if !extra_space.is_false() {}

            if self.local_cib.contains_key(&from_port_name) {
                let announce = self.local_cib.get(&from_port_name).unwrap();
                let exsit_bdd = announce.get_predicate();
                let arrive_space = exsit_bdd.or(&theoretical_space.and(arrive_space));
                let verified_cibtuple = CibTuple::new(arrive_space, 1);
                self.local_cib.insert(from_port_name, verified_cibtuple);
                true
            } else {
                let verified_cibtuple = CibTuple::new(theoretical_space.and(arrive_space), 1);
                self.local_cib.insert(from_port_name, verified_cibtuple);
                return true;
            }
        } else {
            if self.locked_space_port.contains(&space_id) {
                return true;
            } else {
                self.locked_space_port.insert(space_id);
                let verified_cibtuple = CibTuple::new(theoretical_space.clone(), 1);
                self.local_cib.insert(from_port_name, verified_cibtuple);
                return true;
            }
        }
    }

    pub fn count_check_outside_space(
        &mut self,
        dst_device: &Device,
        port_name: String,
        current_ctx: &Ctx,
        visited_devices: &HashSet<String>,
        dst_packet_space_bdd: &Bdd,
        bdd_cache: &mut BddCache,
    ) -> bool {
        let annoucement = current_ctx.get_announcement();
        let from_device_name = current_ctx.get_device_name();

        if !dst_device.has_space_bdd(&port_name) {
            return false;
        }

        if visited_devices.contains(&self.name.to_string()) {
            let space_id = dst_device.get_space_id(&port_name);
            if self.space_id_to_conceptual_space.contains_key(&space_id) {
                return false;
            }
        }

        if !self.update_loc_cib_with_forward_lock_checking(
            dst_device,
            port_name.clone(),
            annoucement,
            dst_packet_space_bdd,
            bdd_cache,
        ) {
            return false;
        }
        true
    }

    pub fn count_check_inside_space(
        &mut self,
        dst_device: &Device,
        port_name: String,
        current_ctx: &Ctx,
        visited_devices: &HashSet<String>,
        dst_packet_space: &Bdd,
        bdd_cache: &mut BddCache,
    ) -> bool {
        let annoucement = current_ctx.get_announcement();
        let from_device_name = current_ctx.get_device_name();

        if !dst_device.has_space_bdd(&port_name) {
            return false;
        }

        if visited_devices.contains(&self.name.to_string()) {
            let space_id = dst_device.get_space_id(&port_name);
            if self.space_id_to_conceptual_space.contains_key(&space_id) {
                return false;
            }
        }

        if !self.update_loc_cib_with_forward_lock_checking(
            dst_device,
            port_name.clone(),
            annoucement,
            dst_packet_space,
            bdd_cache,
        ) {
            return false;
        }
        true
    }

    pub fn count_check_at_interface(
        &mut self,
        port_name: String,
        current_ctx: &Ctx,
        dst_packet_space: &Bdd,
        bdd_cache: &mut BddCache,
    ) -> bool {
        let annoucement = current_ctx.get_announcement();
        let from_device_name = current_ctx.get_device_name();
        let dst_device = self.get_device();

        if !dst_device.has_space_bdd(&port_name) {
            return false;
        }

        if !self.update_loc_cib_with_forward_lock_checking(
            &dst_device,
            port_name.clone(),
            annoucement,
            dst_packet_space,
            bdd_cache,
        ) {
            return false;
        }
        true
    }

    pub fn count_check_with_dst_device(
        &mut self,
        dst_device: &Device,
        port_name: String,
        current_ctx: &Ctx,
        edge_devices: &HashSet<String>,
        visited_devices: &HashSet<String>,
        dst_packet_space_bdd: &Bdd,
        bdd_cache: &mut BddCache,
    ) -> bool {
        let annoucement = current_ctx.get_announcement();
        let from_device_name = current_ctx.get_device_name();

        if !dst_device.has_space_bdd(&port_name) {
            return false;
        }

        let space_id = dst_device.get_space_id(&port_name);
        if self.space_id_to_conceptual_space.contains_key(&space_id) {
            let conceptual_space = self.space_id_to_conceptual_space.get(&space_id).unwrap();
            let arrive_space = annoucement.get_predicate();
            if !conceptual_space.and(arrive_space).is_false() {
                match self.port_arrive_cnt.get_mut(&port_name) {
                    Some(cnt) => {
                        *cnt += 1;
                        if *cnt > 1 {
                            return false;
                        }
                    }
                    None => {
                        self.port_arrive_cnt.insert(port_name.clone(), 1);
                    }
                }
            }
        }

        if !self.update_loc_cib_with_forward_lock_checking(
            dst_device,
            port_name.clone(),
            annoucement,
            dst_packet_space_bdd,
            bdd_cache,
        ) {
            return false;
        }

        if edge_devices.contains(&self.get_name()) {
            return false;
        }
        true
    }

    pub fn count_check_by_space_port(
        &mut self,
        port_name: String,
        current_ctx: &Ctx,
        edge_devices: &HashSet<String>,
        visited_devices: &HashSet<String>,
    ) -> bool {
        let annoucement = current_ctx.get_announcement();

        if !self.port_name_to_space_id.contains_key(&port_name) {
            return false;
        }
        if visited_devices.contains(&self.name.to_string()) {
            return false;
        }

        if self.update_loc_cib_new_by_space_port(port_name.clone(), annoucement) {
            return true;
        }

        if edge_devices.contains(&self.get_name()) {
            return false;
        }
        true
    }

    pub fn get_cib_out(&mut self) -> Announcement {
        let mut count_predicate: HashMap<i32, Bdd> = HashMap::default();
        for cibtuple in self.local_cib.values() {
            let tmp_count = cibtuple.get_count();
            let tmp_bdd = cibtuple.get_predicate();
            if !count_predicate.contains_key(&tmp_count) {
                count_predicate.insert(tmp_count, tmp_bdd.clone());
            } else {
                let new_bdd = count_predicate.get_mut(&tmp_count).unwrap().or(tmp_bdd);
                count_predicate.insert(tmp_count, new_bdd);
            }
        }
        let annoucement_out = Announcement::new(count_predicate.get(&1).unwrap().clone(), 1);
        annoucement_out
    }

    pub fn get_cib_out_predicate(&mut self) -> Bdd {
        let mut count_predicate: HashMap<i32, Bdd> = HashMap::default();
        for cibtuple in self.local_cib.values() {
            let tmp_count = cibtuple.get_count();
            let tmp_bdd = cibtuple.get_predicate();
            if !count_predicate.contains_key(&tmp_count) {
                count_predicate.insert(tmp_count, tmp_bdd.clone());
            } else {
                let new_bdd = count_predicate.get_mut(&tmp_count).unwrap().or(tmp_bdd);
                count_predicate.insert(tmp_count, new_bdd);
            }
        }
        count_predicate.get(&1).unwrap().clone()
    }

    pub fn get_result(&mut self, dst_packet_space_bdd: &Bdd, dst_node_name: &str) -> bool {
        if self.local_cib.len() == 0 {
            false
        } else {
            let annoucement_final = self.get_cib_out();
            let packet_space_bdd_final = annoucement_final.get_predicate();
            let count_final = annoucement_final.get_count();
            if count_final == 1 && packet_space_bdd_final == dst_packet_space_bdd {
                true
            } else {
                false
            }
        }
    }

    pub fn get_result_toward_dst_space_region(
        &mut self,
        dst_space_region: &SpaceNode,
        node_packet_space_table: &Arc<HashMap<String, Bdd>>,
    ) -> (usize, usize) {
        let s0_device_set = dst_space_region.get_s0_nodes_table();
        let s0_len = s0_device_set.len();
        if self.local_cib.len() == 0 {
            (0, s0_len)
        } else {
            let mut reach_cnt = 0;
            let annoucement_final = self.get_cib_out();
            let packet_space_bdd_final = annoucement_final.get_predicate();
            let dst_aggre_packet_space = dst_space_region.get_aggre_space().unwrap();
            if packet_space_bdd_final == dst_aggre_packet_space {
                return (s0_len, 0);
            } else {
                for s0_device_name in s0_device_set.keys() {
                    let s0_packets_space = node_packet_space_table.get(s0_device_name).unwrap();
                    let intersection = packet_space_bdd_final.and(s0_packets_space);
                    if intersection == *s0_packets_space {
                        reach_cnt += 1;
                    } else {
                    }
                }
                return (reach_cnt, s0_len - reach_cnt);
            }
        }
    }
}
