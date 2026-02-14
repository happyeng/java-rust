use super::space_node::SpaceNode;
use crate::util::device_port::DevicePort;
use crate::util::forward_action::ForwardAction;
use crate::util::hash_utils::{HashMap, HashSet};
use crate::util::network::Network;
use crate::util::npbdd::{BDDTable, LayerCache, NPBDD};
use crate::util::pod::Pod;
use crate::util::rule::Rule;
use crate::verifier::device::Device;
use crate::verifier::lec::Lec;
use crate::verifier::node::Node;
use crate::verifier::rule_bdd::RuleBDD;
use biodivine_lib_bdd::*;
use rayon::prelude::*;
use std::mem;
use std::sync::Arc;

#[derive(Clone)]
pub struct Toponet {
    pub ip_bits_len: usize,
    variables_dst_ip: Arc<Vec<BddVariable>>,
    pub variable_set: Arc<BddVariableSet>,
    map_device_rule_bdd: Arc<HashMap<String, HashMap<Rule, RuleBDD>>>,
    map_device_lecs: Arc<HashMap<String, HashSet<Lec>>>,
    pub map_device_packet_space_bdd: Arc<HashMap<String, Bdd>>,
    pub all_space_map: Arc<HashMap<String, Bdd>>,
    pub devices: Arc<HashMap<String, Arc<Device>>>,
    device_space_table: HashMap<String, String>,
    regional_dst_device_bdd_table: HashMap<String, Bdd>,
    pub network: Arc<Network>,
    dst_node_name: String,
    dst_space_node_name: String,
    nodes_table: HashMap<Arc<String>, Node>,
    space_nodes_table: HashMap<String, SpaceNode>,
    space_node_connection: HashMap<String, Bdd>,
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
            map_device_rule_bdd: Arc::new(HashMap::default()),
            map_device_lecs: Arc::new(HashMap::default()),
            map_device_packet_space_bdd: Arc::new(HashMap::default()),
            all_space_map: Arc::new(HashMap::default()),
            devices: Arc::new(HashMap::default()),
            device_space_table: HashMap::default(),
            regional_dst_device_bdd_table: HashMap::default(),
            network: Arc::new(Network::new()),
            dst_node_name: String::new(),
            nodes_table: HashMap::default(),
            dst_space_node_name: String::new(),
            space_node_connection: HashMap::default(),
            space_nodes_table: HashMap::default(),
        }
    }

    pub fn non_arc_size(&self) -> usize {
        mem::size_of_val(&self.ip_bits_len)
            + mem::size_of_val(&self.dst_node_name)
            + mem::size_of_val(&self.nodes_table)
    }

    pub fn reset(&mut self) {
        self.nodes_table.clear();
    }

    pub fn reset_local_cib_within_space(&mut self) {
        for space_node in self.space_nodes_table.values_mut() {
            space_node.reset_all_nodes();
        }
    }

    pub fn get_variables_dst_ip(&self) -> &Vec<BddVariable> {
        &self.variables_dst_ip
    }

    pub fn get_variable_set(&self) -> &BddVariableSet {
        &self.variable_set
    }

    pub fn get_space_node_name(&self) -> String {
        self.dst_space_node_name.clone()
    }

    pub fn set_dst_node_name(&mut self, dst_node_name: String) {
        self.dst_node_name = dst_node_name
    }

    pub fn set_dst_space_node_name(&mut self, dst_space_node_name: String) {
        self.dst_space_node_name = dst_space_node_name;
    }

    pub fn set_arc_devices(&mut self, arc_devices: &Arc<HashMap<String, Arc<Device>>>) {
        self.devices = Arc::clone(&arc_devices);
    }

    pub fn set_arc_network(&mut self, arc_network: &Arc<Network>) {
        self.network = Arc::clone(arc_network);
    }

    pub fn encode_rule_stepwise(
        &mut self,
        tmp_devices: &mut HashMap<String, Device>,
        common_prefix: &String,
        network: &Network,
    ) {
        let all_space = self.all_space_map.values().next().unwrap();
        let all_space_id = BDDTable::insert_bdd(all_space.clone());
        let device_ports_topo = network.get_device_ports();

        tmp_devices
            .par_iter_mut()
            .for_each(|(device_name, device)| {
                let mut port_predicate: HashMap<String, usize> = HashMap::default();
                let topo_ports = match device_ports_topo.get(device_name) {
                    Some(ports) => ports,
                    None => {
                        return;
                    }
                };
                let tmp_rules = device.get_rules();
                if tmp_rules.is_empty() {
                    return;
                }
                let mut all_bdd = self.variable_set.mk_false();
                let mut all_bdd_id = BDDTable::insert_bdd(all_bdd);
                let last_longest_prefix_len = tmp_rules.first().unwrap().get_prefix_len();
                for rule in tmp_rules.iter() {
                    let rule_ip = rule.get_ip();
                    let rule_prefix_len = rule.get_prefix_len();
                    if !rule_ip.starts_with(common_prefix) && rule_prefix_len != 0 {
                        continue;
                    }

                    let prefix_key = format!("{}/{}", rule_ip, rule_prefix_len);
                    let entry = BDDTable::get_prefix_bdd_map()
                        .get(&prefix_key)
                        .map(|cached| *cached.value());
                    let bdd_match_id = if let Some(cached_id) = entry {
                        cached_id
                    } else {
                        let result = NPBDD::make_prefix_bdd(rule.get_ip(), rule.get_prefix_len());
                        let new_bdd_id = BDDTable::insert_bdd(result.clone());
                        BDDTable::get_prefix_bdd_map().insert(prefix_key, new_bdd_id);
                        new_bdd_id
                    };

                    let is_relevant = LayerCache::cached_relevance(all_space_id, bdd_match_id);
                    if !is_relevant {
                        continue;
                    }

                    // 2. calculate hit
                    let mut bdd_hit_id = bdd_match_id;
                    if rule.get_prefix_len() == last_longest_prefix_len {
                        all_bdd_id = LayerCache::cached_or(all_bdd_id, bdd_hit_id);
                    } else {
                        (all_bdd_id, bdd_hit_id) =
                            LayerCache::cached_prefix_match(all_bdd_id, bdd_hit_id);
                    }

                    if BDDTable::get_bdd_by_id(bdd_hit_id).unwrap().is_false() {
                        continue;
                    }

                    // 3. calculate lec
                    let forward_action = rule.get_forward_action();
                    let tmp_ports = forward_action.get_ports();
                    let mut last_calculation_pair: Option<(u32, u32)> = None;
                    for port in tmp_ports {
                        let tmp_device_port =
                            DevicePort::new(device_name.to_string(), port.to_string());
                        if !topo_ports.contains(&tmp_device_port) {
                            continue;
                        }
                        if let Some(&old_predicate_id) = port_predicate.get(port) {
                            // 实现 cached_lec_merge 的逻辑
                            let old_predicate_id_u32: u32 = old_predicate_id.try_into().unwrap();
                            if let Some((last_old_predicate_id, last_result_predicate_id)) =
                                last_calculation_pair
                            {
                                if old_predicate_id_u32 == last_old_predicate_id {
                                    last_calculation_pair =
                                        Some((last_old_predicate_id, last_result_predicate_id));
                                } else {
                                    let new_predicate_id =
                                        LayerCache::cached_or(old_predicate_id_u32, bdd_hit_id);
                                    last_calculation_pair =
                                        Some((old_predicate_id_u32, new_predicate_id));
                                }
                            } else {
                                let new_predicate_id =
                                    LayerCache::cached_or(old_predicate_id_u32, bdd_hit_id);
                                last_calculation_pair =
                                    Some((old_predicate_id_u32, new_predicate_id));
                            }
                            if let Some((_, new_predicate_id)) = last_calculation_pair {
                                port_predicate
                                    .insert(port.to_string(), new_predicate_id.try_into().unwrap());
                            }
                        } else {
                            port_predicate.insert(port.to_string(), bdd_hit_id.try_into().unwrap());
                        }
                    }
                }
                let mut tmp_lecs = HashSet::default();
                for (port, predicate_id) in port_predicate.into_iter() {
                    tmp_lecs.insert(Lec::new(
                        ForwardAction::new("ALL".to_owned(), vec![port.clone()]),
                        BDDTable::get_bdd_by_id(predicate_id.try_into().unwrap()).unwrap(),
                    ));
                }
                device.cal_forwardable_space(&tmp_lecs);
                device.merge_lec_to_space_port(tmp_lecs);
            });
    }

    pub fn encode_rule_npbdd(
        &mut self,
        tmp_devices: &mut HashMap<String, Device>,
        common_prefix: &String,
        network: &Network,
    ) {
        let all_space = self.all_space_map.values().next().unwrap();
        let all_space_id = BDDTable::insert_bdd(all_space.clone());
        let device_ports_topo = network.get_device_ports();

        tmp_devices
            .par_iter_mut()
            .for_each(|(device_name, device)| {
                let mut port_predicate: HashMap<String, u32> = HashMap::default();
                let topo_ports = match device_ports_topo.get(device_name) {
                    Some(ports) => ports,
                    None => {
                        return;
                    }
                };
                let tmp_rules = device.get_rules();
                if tmp_rules.is_empty() {
                    return;
                }
                // used_space (fwded) 初始化为 false (⊥)，并复用 false_id 以避免重复构造。
                let false_id = BDDTable::insert_bdd(self.variable_set.mk_false());
                let mut used_space_id = false_id;

                for rule in tmp_rules.iter() {
                    let rule_ip = rule.get_ip();
                    let rule_prefix_len = rule.get_prefix_len();
                    if !rule_ip.starts_with(common_prefix) && rule_prefix_len != 0 {
                        continue;
                    }

                    let prefix_bdd_id = LayerCache::l2_encode_rule(rule_ip, rule_prefix_len);

                    let is_relevant = LayerCache::cached_relevance(all_space_id, prefix_bdd_id);
                    if !is_relevant {
                        continue;
                    }

                    let (hit_id, new_used_space_id) =
                        LayerCache::l2_cal_hit(prefix_bdd_id, used_space_id);
                    used_space_id = new_used_space_id;

                    if BDDTable::get_bdd_by_id(hit_id).unwrap().is_false() {
                        continue;
                    }

                    let forward_action = rule.get_forward_action();
                    let tmp_ports = forward_action.get_ports();
                    let mut port_ids_to_update: Vec<u32> = Vec::new();
                    let mut ports_to_update: Vec<String> = Vec::new();

                    for port in tmp_ports {
                        let tmp_device_port =
                            DevicePort::new(device_name.to_string(), port.to_string());
                        if !topo_ports.contains(&tmp_device_port) {
                            continue;
                        }
                        ports_to_update.push(port.clone());
                        if let Some(&old_port_id) = port_predicate.get(port) {
                            port_ids_to_update.push(old_port_id);
                        } else {
                            // 新端口，初始化为 false，然后与 hit 合并
                            port_ids_to_update.push(false_id);
                        }
                    }

                    if !port_ids_to_update.is_empty() {
                        let new_port_ids =
                            LayerCache::l2_merge_port_space(hit_id, &port_ids_to_update);
                        for (port, &new_port_id) in ports_to_update.iter().zip(new_port_ids.iter())
                        {
                            port_predicate.insert(port.clone(), new_port_id);
                        }
                    }
                }

                let mut tmp_lecs = HashSet::default();
                for (port, predicate_id) in port_predicate.into_iter() {
                    tmp_lecs.insert(Lec::new(
                        ForwardAction::new("ALL".to_owned(), vec![port.clone()]),
                        BDDTable::get_bdd_by_id(predicate_id).unwrap(),
                    ));
                }
                device.cal_forwardable_space(&tmp_lecs);
                device.merge_lec_to_space_port(tmp_lecs);
            });
    }

    pub fn encode_packet_space_group(
        &mut self,
        tmp_devices: &mut HashMap<String, Device>,
        dst_devices: &HashSet<String>,
    ) {
        let tmp_map_device_subnet_bdd: HashMap<String, Bdd> = dst_devices
            .par_iter()
            .filter_map(|dst_device_name| {
                let edge_device = tmp_devices.get(dst_device_name).expect(&format!(
                    "Failed to find device with name: {}",
                    dst_device_name
                ));
                if let Some(packet_space) = edge_device.get_packet_space() {
                    let bdd = NPBDD::make_prefix_bdd(
                        packet_space.get_ip(),
                        packet_space.get_prefix_len(),
                    );
                    Some((dst_device_name.clone(), bdd))
                } else {
                    None
                }
            })
            .collect();

        tmp_devices
            .par_iter_mut()
            .for_each(|(device_name, device)| {
                if let Some(packet_space) = device.get_packet_space() {
                    device.subnet_space = NPBDD::make_prefix_bdd(
                        packet_space.get_ip(),
                        packet_space.get_prefix_len(),
                    );
                }
            });

        if let Some((_, bdd)) = tmp_map_device_subnet_bdd.iter().next() {
            let all_space = tmp_map_device_subnet_bdd
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .into_par_iter()
                .reduce(|| bdd.clone(), |acc, cur| acc.or(&cur));

            let mut all_space_map: HashMap<String, Bdd> = HashMap::default();
            all_space_map.insert("All".to_string(), all_space);
            self.all_space_map = Arc::new(all_space_map);
        } else {
            println!("HashMap is empty");
        }
        self.map_device_packet_space_bdd = Arc::new(tmp_map_device_subnet_bdd);
    }

    fn get_tmatch(&self, device_name: &str, rule: &Rule) -> Option<Bdd> {
        self.map_device_rule_bdd
            .get(device_name)?
            .get(rule)
            .map(|rule_bdd| rule_bdd.get_match().clone())
    }

    fn get_packet_space(&self) -> &Bdd {
        self.map_device_packet_space_bdd
            .get(&self.dst_node_name)
            .unwrap()
    }

    fn get_space_node_bdd(&self) -> &Bdd {
        let space_node = self
            .space_nodes_table
            .get(&self.dst_space_node_name)
            .unwrap();
        space_node.get_aggre_space().unwrap()
    }

    pub fn gen_topo_node(&mut self, devices_name: &Vec<Arc<String>>) {
        for device_name in devices_name {
            let mut node = Node::new(device_name.clone());
            let device = self.devices.get(&device_name.to_string()).unwrap();
            node.set_device(Arc::clone(device));
            self.nodes_table.insert(device_name.clone(), node);
        }
    }

    pub fn gen_topo_node_outside_space(
        &mut self,
        devices_name: &Vec<Arc<String>>,
        pod_device_names: &HashSet<String>,
    ) {
        for device_name in devices_name {
            if pod_device_names.contains(&device_name.to_string()) {
                continue;
            }
            let device = self.devices.get(&device_name.to_string()).unwrap();
            let mut node = Node::new(device_name.clone());
            node.set_device(Arc::clone(device));
            self.nodes_table
                .insert(device_name.to_string().into(), node);
        }
    }

    pub fn gen_space_node(&mut self, pods: &HashMap<i32, Pod>, edge_devices: &HashSet<String>) {
        for (pod_id, pod) in pods {
            let mut cur_space_node = SpaceNode::new(pod_id.to_string());
            cur_space_node.gen_internal_node(pod, &self.devices, edge_devices);
            self.space_nodes_table
                .insert(pod_id.to_string(), cur_space_node);
        }
        if let Some(dst_space_node) = self.space_nodes_table.get_mut(&self.dst_space_node_name) {
            dst_space_node.aggregate_packet_space(
                &self.map_device_packet_space_bdd,
                &mut self.regional_dst_device_bdd_table,
            );
        };
    }

    pub fn reset_node(&mut self) {
        for node in self.nodes_table.values_mut() {
            node.reset_node();
        }
    }

    pub fn compare_with_other_map_device_rule_bdd(&self, o_toponet: Toponet) {
        let other_map_device = o_toponet.get_map_device_rule_bdd();
        for (device_name, map_bdd) in &*self.map_device_rule_bdd {
            let other_map = other_map_device.get(device_name);
            for (rule, rule_bdd) in map_bdd {
                let other_rule_bdd = other_map.expect("REASON").get(rule);
                let res = rule_bdd.compare_with_other_rule_bdd(other_rule_bdd.unwrap());
                if !res {
                    println!("Rule {} not equal", rule.get_name());
                } else {
                    // println!("{} correct!!!",rule.get_name());
                }
            }
        }
    }

    pub fn get_map_device_rule_bdd(&self) -> &HashMap<String, HashMap<Rule, RuleBDD>> {
        &self.map_device_rule_bdd
    }
}
