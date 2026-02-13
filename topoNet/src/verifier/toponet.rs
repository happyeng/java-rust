use crate::util::forward_action::ForwardAction;
use crate::util::network::Network;
use crate::util::rule::Rule;
use crate::verifier::bdd_engine::BddEngine;
use crate::verifier::context::Ctx;
use crate::verifier::device::Device;
use crate::verifier::lec::Lec;
use crate::verifier::node::Node;
use crate::verifier::rule_bdd::RuleBDD;
use biodivine_lib_bdd::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct Toponet {
    ip_bits_len: usize,
    variables_dst_ip: Arc<Vec<BddVariable>>,
    variable_set: Arc<BddVariableSet>,
    map_device_lecs: Arc<HashMap<String, HashSet<Lec>>>,
    map_device_packet_space_bdd: Arc<HashMap<String, Bdd>>,
    devices: Arc<HashMap<String, Device>>,
    network: Arc<Network>,
    dst_node_name: String,
    nodes_table: HashMap<String, Node>,
}

impl Toponet {
    pub fn new(ip_bits: usize) -> Self {
        let mut variable_builder = BddVariableSetBuilder::new();
        let mut variables_dst_ip = Vec::new();

        for i in 0..ip_bits {
            let var_name: String = format!("x{}", i + 1);
            let var = variable_builder.make_variable(&var_name);
            variables_dst_ip.push(var);
        }

        let variable_set = variable_builder.build();

        Toponet {
            ip_bits_len: ip_bits,
            variables_dst_ip: Arc::new(variables_dst_ip),
            variable_set: Arc::new(variable_set),
            map_device_lecs: Arc::new(HashMap::new()),
            map_device_packet_space_bdd: Arc::new(HashMap::new()),
            devices: Arc::new(HashMap::new()),
            network: Arc::new(Network::new()),
            dst_node_name: String::new(),
            nodes_table: HashMap::new(),
        }
    }

    pub fn get_variables_dst_ip(&self) -> &Vec<BddVariable> {
        &self.variables_dst_ip
    }
    pub fn get_variable_set(&self) -> &BddVariableSet {
        &self.variable_set
    }
    pub fn set_dst_node_name(&mut self, dst_node_name: String) {
        self.dst_node_name = dst_node_name
    }
    pub fn set_arc_devices(&mut self, arc_devices: &Arc<HashMap<String, Device>>) {
        self.devices = Arc::clone(&arc_devices);
    }
    pub fn set_arc_network(&mut self, arc_network: &Arc<Network>) {
        self.network = Arc::clone(arc_network);
    }

    #[allow(dead_code)]
    fn update_device_lec(
        &self,
        device: &Device,
        match_bdd_map: &HashMap<Rule, RuleBDD>,
    ) -> HashSet<Lec> {
        let rules = device.get_rules();
        let mut port_predicate: HashMap<ForwardAction, Bdd> = HashMap::new();
        for rule in rules {
            let forward_action = &rule.get_forward_action();
            let tmp_ports = forward_action.get_ports();
            for port in tmp_ports {
                let cur_forward_action = ForwardAction::new(
                    forward_action.get_forward_type().clone(),
                    vec![port.clone()],
                );
                if port_predicate.contains_key(&cur_forward_action) {
                    let old_predicate = port_predicate.get(&cur_forward_action).cloned().unwrap();
                    let new_hit = match_bdd_map.get(rule).unwrap().get_hit().clone();
                    let new_predicate = old_predicate.or(&new_hit);
                    port_predicate.insert(cur_forward_action.clone(), new_predicate);
                } else {
                    port_predicate.insert(
                        cur_forward_action.clone(),
                        match_bdd_map.get(&rule).clone().unwrap().get_hit().clone(),
                    );
                }
            }
        }
        let mut tmp_lecs = HashSet::new();
        for (forward_action, predicate) in &port_predicate {
            tmp_lecs.insert(Lec::new(forward_action.clone(), predicate.clone()));
        }
        tmp_lecs
    }

    pub fn encode_rule_to_lec(&mut self) {
        let tmp_map_device_lecs = self
            .devices
            .par_iter()
            .map(|(device_name, device)| {
                let mut port_predicate: HashMap<ForwardAction, Bdd> = HashMap::default();
                let tmp_rules = device.get_rules();
                let mut set_first = false;
                let mut all_bdd = self.variable_set.mk_true();
                for rule in tmp_rules.into_iter() {
                    let bdd_match: Bdd = BddEngine::encode_dst_ip_prefix(
                        rule.get_ip(),
                        rule.get_prefix_len(),
                        self.get_variables_dst_ip(),
                        self.get_variable_set(),
                        self.ip_bits_len,
                    );
                    let mut bdd_hit = bdd_match.clone();
                    if !set_first {
                        set_first = true;
                        all_bdd = bdd_match.clone();
                    } else {
                        let tmp = all_bdd.not();
                        bdd_hit = bdd_match.and(&tmp);
                        all_bdd = all_bdd.or(&bdd_match);
                    }
                    let forward_action = &rule.get_forward_action();
                    let tmp_ports = forward_action.get_ports();
                    for port in tmp_ports {
                        let cur_forward_action = ForwardAction::new(
                            forward_action.get_forward_type().clone(),
                            vec![port.clone()],
                        );
                        if port_predicate.contains_key(&cur_forward_action) {
                            let old_predicate =
                                port_predicate.get(&cur_forward_action).cloned().unwrap();
                            let new_hit = bdd_hit.clone();
                            let new_predicate = old_predicate.or(&new_hit);
                            port_predicate.insert(cur_forward_action.clone(), new_predicate);
                        } else {
                            port_predicate.insert(cur_forward_action.clone(), bdd_hit.clone());
                        }
                    }
                }
                let mut tmp_lecs = HashSet::default();
                for (forward_action, predicate) in port_predicate.into_iter() {
                    tmp_lecs.insert(Lec::new(forward_action, predicate));
                }
                (device_name.clone(), tmp_lecs)
            })
            .collect();
        self.map_device_lecs = Arc::new(tmp_map_device_lecs);
    }

    pub fn encode_packet_space(&mut self) {
        let start: Instant = Instant::now();
        let tmp_map_device_subnet_bdd: HashMap<String, Bdd> = self
            .devices
            .par_iter()
            .filter_map(|(device_name, device)| {
                if let Some(packet_space) = device.get_packet_space() {
                    let bdd: Bdd = BddEngine::encode_dst_ip_prefix(
                        packet_space.get_ip(),
                        packet_space.get_prefix_len(),
                        self.get_variables_dst_ip(),
                        self.get_variable_set(),
                        self.ip_bits_len,
                    );
                    Some((device_name.clone(), bdd))
                } else {
                    None
                }
            })
            .collect();
        self.map_device_packet_space_bdd = Arc::new(tmp_map_device_subnet_bdd);
        let _duration: std::time::Duration = start.elapsed();
    }

    fn get_packet_space(&self) -> &Bdd {
        self.map_device_packet_space_bdd
            .get(&self.dst_node_name)
            .unwrap()
    }

    pub fn gen_topo_node(&mut self, devices_name: &Vec<String>) {
        for device_name in devices_name {
            let node = Node::new(device_name.clone());
            self.nodes_table.insert(device_name.clone(), node);
        }
    }

    pub fn node_cal_in_degree(&mut self) {
        for node in self.nodes_table.values_mut() {
            if node.get_name() == self.dst_node_name {
                continue;
            }
            if let Some(_value) = self.map_device_packet_space_bdd.get(&self.dst_node_name) {
            } else {
                // not found
            };
            let packet_space_bdd = self
                .map_device_packet_space_bdd
                .get(&self.dst_node_name)
                .cloned()
                .unwrap();
            let device_lecs = self.map_device_lecs.get(&node.get_name()).unwrap();
            node.init_cib(
                self.network.as_ref(),
                packet_space_bdd,
                device_lecs,
                self.dst_node_name.clone(),
            );
        }
    }

    pub fn start_count(&mut self, network: &Network, edge_devices: &HashSet<String>) {
        self.bfs(network, edge_devices);
    }

    pub fn show_result(&mut self, edge_devices: &HashSet<String>) {
        let dst_packet_space_bdd = self
            .map_device_packet_space_bdd
            .get(&self.dst_node_name)
            .unwrap();
        let mut reach_cnt = 0;
        let mut unreach_cnt = 0;
        for node in self.nodes_table.values_mut() {
            let cur_node_name = node.get_name();
            if edge_devices.contains(&cur_node_name) && cur_node_name != self.dst_node_name {
                let res = node.get_result(dst_packet_space_bdd);
                if res == false {
                    unreach_cnt += 1;
                } else if res == true {
                    reach_cnt += 1;
                }
            }
        }
        let _ = (reach_cnt, unreach_cnt);
    }

    pub fn bfs(&mut self, network: &Network, edge_devices: &HashSet<String>) {
        let device_ports_topo = network.get_device_ports();
        let topology = network.get_toplogy();
        let start_context = Ctx::new(
            self.dst_node_name.clone(),
            self.get_packet_space().clone(),
            1,
        );
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<Ctx> = VecDeque::new();
        queue.push_back(start_context);
        let mut bfs_cnt = 0;
        let mut ctx_cnt = 0;
        let mut check_cnt = 0;

        while !queue.is_empty() {
            bfs_cnt += 1;
            let size = queue.len();
            let mut indegree_check_set: HashSet<String> = HashSet::new();
            for _ in 0..size {
                if let Some(current_ctx) = queue.pop_front() {
                    let cur_device_name = current_ctx.get_device_name().clone();
                    visited.insert(cur_device_name.clone());
                    let mut _satisfied_count = 0;
                    if let Some(cur_ports) = device_ports_topo.get(&cur_device_name) {
                        for cur_port in cur_ports {
                            if let Some(dst_port) = topology.get(cur_port) {
                                let dst_device_name = dst_port.get_device_name();
                                check_cnt += 1;
                                if !visited.contains(&dst_device_name) {
                                    let dst_node =
                                        self.nodes_table.get_mut(&dst_device_name).unwrap();
                                    indegree_check_set.insert(dst_device_name.clone());
                                    if dst_node.count_check(
                                        dst_port.get_port_name(),
                                        &current_ctx,
                                        edge_devices,
                                    ) {
                                        ctx_cnt += 1;
                                        _satisfied_count += 1;
                                        let cib_out_announcement = dst_node.get_cib_out();
                                        let new_ctx = Ctx::new(
                                            dst_device_name.clone(),
                                            cib_out_announcement.get_predicate().clone(),
                                            1,
                                        );
                                        queue.push_back(new_ctx);
                                        visited.insert(dst_device_name);
                                    } else {
                                        // not satisfied
                                    }
                                } else {
                                    // already visited
                                }
                            } else {
                                // topo missing edge
                            }
                        }
                    } else {
                        // no port info
                    }
                }
            }
            for dst_device_name in indegree_check_set {
                let dst_node = self.nodes_table.get_mut(&dst_device_name).unwrap();
                let tmp_lec_cnt = dst_node.get_indegree_lec_cnt();
                if tmp_lec_cnt > 0 {}
            }
        }
        let _ = (bfs_cnt, ctx_cnt, check_cnt);
    }
}
