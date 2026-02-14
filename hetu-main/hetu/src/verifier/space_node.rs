use super::device::Device;
use super::{annoucement::Announcement, cibtuple::CibTuple, context::Ctx, node::Node};
use crate::util::hash_utils::{HashMap, HashSet};
use crate::util::{device_port::DevicePort, pod::Pod};
use biodivine_lib_bdd::Bdd;
use std::sync::Arc;

#[derive(Clone)]
pub struct SpaceNode {
    name: String,
    interfaces: HashSet<DevicePort>,
    s1_nodes_table: HashMap<String, Node>,
    s0_nodes_table: HashMap<String, Node>,
    aggre_space: Option<Bdd>,
    local_s1_cib: HashMap<String, CibTuple>,
}

impl SpaceNode {
    pub fn new(name: String) -> SpaceNode {
        SpaceNode {
            name,
            interfaces: HashSet::default(),
            s1_nodes_table: HashMap::default(),
            s0_nodes_table: HashMap::default(),
            aggre_space: None,
            local_s1_cib: HashMap::default(),
        }
    }

    pub fn aggregate_packet_space(
        &mut self,
        packet_space_map: &HashMap<String, Bdd>,
        regional_dst_node_bdd_table: &mut HashMap<String, Bdd>,
    ) {
        let mut aggre_packet_space: Option<Bdd> = None;
        for s0_device_name in self.s0_nodes_table.keys() {
            if let Some(packet_space) = packet_space_map.get(s0_device_name) {
                regional_dst_node_bdd_table
                    .insert(s0_device_name.to_string(), packet_space.clone());
                aggre_packet_space = match aggre_packet_space {
                    Some(existing_space) => Some(existing_space.or(packet_space)),
                    None => Some(packet_space.clone()),
                };
            } else {
            }
        }

        self.aggre_space = aggre_packet_space.clone();
        if let Some(aggregated) = aggre_packet_space {
            for s0_device_name in self.s0_nodes_table.keys() {
                if let Some(packet_space) = packet_space_map.get(s0_device_name) {
                    let intersection = aggregated.and(packet_space);
                    if intersection != *packet_space {
                        panic!("Validation failed: the subnet of device {} does not match the aggregated subnet", s0_device_name);
                    }
                }
            }
        }
    }

    pub fn get_aggre_space(&self) -> Option<&Bdd> {
        self.aggre_space.as_ref()
    }

    pub fn gen_internal_node(
        &mut self,
        pod: &Pod,
        devices: &Arc<HashMap<String, Arc<Device>>>,
        edge_devices: &HashSet<String>,
    ) {
        let s0_device_names = pod.get_s0_device_names();
        let s1_device_names = pod.get_s1_device_names();
        for s0_device_name in s0_device_names {
            if !edge_devices.contains(s0_device_name) {
                continue;
            }
            let mut node = Node::new(s0_device_name.clone().into());
            let device = devices.get(s0_device_name).unwrap();
            node.set_device(device.clone());
            self.s0_nodes_table.insert(s0_device_name.to_string(), node);
        }
        for s1_device_name in s1_device_names {
            let mut node = Node::new(s1_device_name.clone().into());
            let device = devices.get(s1_device_name).unwrap();
            node.set_device(device.clone());
            self.s1_nodes_table.insert(s1_device_name.to_string(), node);
        }
    }

    pub fn set_interfaces(&mut self, device_ports: &HashMap<String, HashSet<DevicePort>>) {
        for (s1_device_name, _node) in &self.s1_nodes_table {
            if let Some(ports) = device_ports.get(s1_device_name) {
                for port in ports {
                    if let Some((peer_device_name, _peer_port_name)) = port.get_peer_port() {
                        if !self.s0_nodes_table.contains_key(peer_device_name) {
                            self.interfaces.insert(port.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn check_s1_arrived_space(&self, s1_device_name: &String) -> bool {
        self.local_s1_cib.contains_key(s1_device_name)
    }

    pub fn get_s1_arrive_space(&self, s1_device_name: &String) -> &Bdd {
        let cibtuple = self.local_s1_cib.get(s1_device_name).unwrap();
        cibtuple.get_predicate()
    }

    pub fn update_local_cib(&mut self, arrive_s1_device_name: &String, current_ctx: &Ctx) {
        let annoucement = current_ctx.get_announcement();
        let arrive_space = annoucement.get_predicate();
        let new_cibtuple: CibTuple;
        if let Some(verified_cibtuple) = self.local_s1_cib.get(arrive_s1_device_name) {
            let verified_space = verified_cibtuple.get_predicate();
            let new_space = verified_space.or(arrive_space);
            new_cibtuple = CibTuple::new(new_space, 1);
        } else {
            new_cibtuple = CibTuple::new(arrive_space.clone(), 1);
        }
        self.local_s1_cib
            .insert(arrive_s1_device_name.clone(), new_cibtuple);
    }

    pub fn get_cib_out(&self) -> Announcement {
        let mut count_predicate: HashMap<i32, Bdd> = HashMap::default();
        for cibtuple in self.local_s1_cib.values() {
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

    pub fn has_local_cib(&self) -> bool {
        !self.local_s1_cib.is_empty()
    }

    pub fn reset_all_nodes(&mut self) {
        for s0_node in self.get_s0_nodes_table_mut().values_mut() {
            s0_node.reset_node()
        }
        for s1_node in self.get_s1_nodes_table_mut().values_mut() {
            s1_node.reset_node()
        }
    }

    pub fn is_s0_node(&self, node_name: &String) -> bool {
        self.s0_nodes_table.contains_key(node_name)
    }

    pub fn is_s1_node(&self, node_name: &String) -> bool {
        self.s1_nodes_table.contains_key(node_name)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_s1_nodes_table(&self) -> &HashMap<String, Node> {
        &self.s1_nodes_table
    }

    pub fn get_s0_nodes_table(&self) -> &HashMap<String, Node> {
        &self.s0_nodes_table
    }

    pub fn get_s0_nodes_table_mut(&mut self) -> &mut HashMap<String, Node> {
        &mut self.s0_nodes_table
    }

    pub fn get_s1_nodes_table_mut(&mut self) -> &mut HashMap<String, Node> {
        &mut self.s1_nodes_table
    }
}
