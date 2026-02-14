use crate::util::network::Network;
use crate::util::npbdd::NPBDD;
use crate::util::rule::Rule;
use crate::verifier::device::Device;
use crate::verifier::neighborhood::{Neighborhood, PacketSpaceAwareDevice};
use crate::verifier::npnet::{InvariantType, NPNet, TraversalType};
use crate::verifier::toponet::Toponet;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use crate::util::hash_utils::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
struct packet {
    prefix: String,
    prefix_len: usize,
    host_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct SubNet {
    pub prefix: IpAddr,
    pub prefix_len: u8,
}

impl SubNet {
    pub fn new(prefix: IpAddr, prefix_len: u8) -> Self {
        SubNet { prefix, prefix_len }
    }
}

pub struct Simulator {
    file_dir: String,
    devices_name: Vec<Arc<String>>,
    pub devices: Arc<HashMap<String, Arc<Device>>>,
    edge_devices: HashSet<String>,
    dst_devices: HashSet<String>,
    pub src_toponet: Toponet,
    ip_bits: usize,
    pub network: Arc<Network>,
    common_prefix: String,
}

impl Simulator {
    pub fn new(ip_bits: usize) -> Self {
        let num_cpu = num_cpus::get();
        println!("Number of logical cores: {}", num_cpu);
        NPBDD::init(ip_bits);
        Simulator {
            file_dir: String::new(),
            devices_name: Vec::new(),
            devices: Arc::new(HashMap::default()),
            edge_devices: HashSet::default(),
            dst_devices: HashSet::default(),
            src_toponet: Toponet::new(ip_bits),
            ip_bits,
            network: Arc::new(Network::new()),
            common_prefix: String::new(),
        }
    }

    pub fn set_file_dir(&mut self, file_dir: &str) {
        self.file_dir = file_dir.to_string();
    }

    pub fn get_devices_name(&mut self) {
        let routes_dir: String = format!("{}/routes", self.file_dir);
        let entries = fs::read_dir(routes_dir).expect("Failed to read directory");
        let mut tmp_devices_name: Vec<Arc<String>> = Vec::new();
        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let file_name = entry.file_name();
            let device_name = file_name.to_string_lossy().to_string();
            tmp_devices_name.push(device_name.into());
        }
        self.devices_name = tmp_devices_name;
    }

    pub fn convert_subnet_devices_to_packets(
        subnet_devices: HashMap<String, Vec<SubNet>>,
    ) -> Vec<packet> {
        let mut packets = Vec::new();
        for (host_name, subnets) in subnet_devices {
            for subnet in subnets {
                let packet = packet {
                    prefix: subnet.prefix.to_string(),
                    prefix_len: subnet.prefix_len as usize,
                    host_name: host_name.clone(),
                };
                packets.push(packet);
            }
        }
        packets
    }

    pub fn read_devices_files_and_encode(&mut self) {
        let mut tmp_devices = self.generate_devices_and_read_rules();
        let packets = self.read_packet_space();
        self.set_packet_space(&mut tmp_devices, packets);
        self.src_toponet
            .encode_packet_space_group(&mut tmp_devices, &self.dst_devices);
        self.src_toponet
            .encode_rule_npbdd(&mut tmp_devices, &self.common_prefix, &self.network);
        self.assign_device_id_and_encode(&mut tmp_devices);
        self.set_arc_devices(tmp_devices);
    }

    fn generate_devices_and_read_rules(&self) -> HashMap<String, Device> {
        self.devices_name
            .par_iter()
            .map(|device_name_arc| {
                let device_name = Arc::clone(device_name_arc);
                let mut tdevice: Device = Device::new((*device_name).clone());
                let rule_file_path = format!("{}/routes/{}", self.file_dir, device_name);
                tdevice.read_rules_file(&rule_file_path);
                (
                    Arc::try_unwrap(device_name).unwrap_or_else(|arc| (*arc).clone()),
                    tdevice,
                )
            })
            .collect()
    }

    fn read_packet_space(&self) -> Vec<packet> {
        let packet_space_file_path = format!("{}/packet_space.json", self.file_dir);
        let contents =
            fs::read_to_string(packet_space_file_path).expect("Error while reading the file");
        let packet_space_input: Value =
            serde_json::from_str(&contents).expect("Error while parsing the JSON");
        match packet_space_input {
            Value::Array(array) => {
                serde_json::from_value(Value::Array(array)).expect("error parsing array")
            }
            Value::Object(map) => {
                let subnet_devices: HashMap<String, Vec<SubNet>> =
                    serde_json::from_value(Value::Object(map)).expect("Failed to parse JSON");
                Self::convert_subnet_devices_to_packets(subnet_devices)
            }
            _ => Vec::new(),
        }
    }

    fn set_packet_space(
        &mut self,
        tmp_devices: &mut HashMap<String, Device>,
        packets: Vec<packet>,
    ) {
        let mut prefixes: Vec<String> = Vec::new();
        for packet in packets {
            let tmp_device_name = packet.host_name.clone();
            let packet_space = Rule::new_for_packet_space(packet.prefix_len, packet.prefix.clone());
            prefixes.push(packet.prefix);
            if let Some(device) = tmp_devices.get_mut(&tmp_device_name) {
                device.set_packet_space_file(packet_space);
            } else {
                println!("Failed to find packet space device: {:?}", tmp_device_name);
            }
        }
        self.common_prefix = Self::find_common_prefix(&prefixes);
    }

    fn set_arc_devices(&mut self, tmp_devices: HashMap<String, Device>) {
        let arc_tmp_devices: HashMap<String, Arc<Device>> = tmp_devices
            .into_iter()
            .map(|(key, device)| (key, Arc::new(device)))
            .collect();

        self.devices = Arc::new(arc_tmp_devices);
        self.src_toponet.set_arc_devices(&self.devices);
    }

    fn find_common_prefix(prefixes: &[String]) -> String {
        if prefixes.is_empty() {
            return String::new();
        }
        let mut common_prefix = prefixes[0].clone();
        for prefix in prefixes.iter().skip(1) {
            let mut temp_prefix = String::new();
            for (c1, c2) in common_prefix.chars().zip(prefix.chars()) {
                if c1 == c2 {
                    temp_prefix.push(c1);
                } else {
                    break;
                }
            }
            common_prefix = temp_prefix;
            if common_prefix.is_empty() {
                break;
            }
        }
        common_prefix
    }

    pub fn get_edge_devices_name(&mut self) {
        let edge_device_file_path = format!("{}/edge_devices", self.file_dir);
        let edge_device_file = File::open(edge_device_file_path);
        let reader = BufReader::new(edge_device_file.unwrap());
        for edge_device_name in reader.lines() {
            self.edge_devices.insert(edge_device_name.unwrap());
        }
    }

    pub fn get_dst_devices_name(&mut self) {
        let dst_device_file_path = format!("{}/edge_devices", self.file_dir);
        let dst_device_file = File::open(dst_device_file_path);
        let reader = BufReader::new(dst_device_file.unwrap());
        for dst_device_name in reader.lines() {
            self.dst_devices.insert(dst_device_name.unwrap());
        }
    }

    pub fn init_network(&mut self) {
        let mut tmp_network = Network::new();
        let topology_filepath = format!("{}/topology.json", self.file_dir);
        tmp_network.read_topology_by_file(&topology_filepath);
        self.network = Arc::new(tmp_network);
        self.src_toponet.set_arc_network(&self.network);
    }

    pub fn build(&mut self) {
        self.get_devices_name();
        self.get_edge_devices_name();
        self.get_dst_devices_name();
        self.init_network();
        self.read_devices_files_and_encode();
    }
}

impl Simulator {
    pub fn assign_device_id_and_encode(&self, tmp_devices: &mut HashMap<String, Device>) {
        let mut device_id = 0;
        for (_, device) in tmp_devices.iter_mut() {
            device.device_id = device_id;
            device_id += 1;
        }
        tmp_devices.par_iter_mut().for_each(|(_, device)| {
            device.device_id_bdd = NPBDD::make_src_device_bdd(device.device_id);
        });
    }

    pub fn find_neighborhood_from_subnet_space(&self) -> Vec<Neighborhood> {
        let mut marked_nodes = HashMap::default();
        self.src_toponet
            .map_device_packet_space_bdd
            .iter()
            .for_each(|(device_name, space_bdd)| {
                let device_id = self.devices.get(device_name).unwrap().device_id;
                let packet_space_aware_device =
                    PacketSpaceAwareDevice::new(device_name.clone(), space_bdd.clone(), device_id);
                marked_nodes.insert(device_name.clone(), packet_space_aware_device);
            });
        let neighborhoods = self.two_hops_merge(marked_nodes);
        neighborhoods
    }

    fn two_hops_merge(
        &self,
        marked_nodes: HashMap<String, PacketSpaceAwareDevice>,
    ) -> Vec<Neighborhood> {
        let mut visited: HashMap<String, bool> = HashMap::default();
        let mut components: Vec<Neighborhood> = Vec::new();
        let proximity_depth = 2;
        for (device_name, device) in marked_nodes.iter() {
            match visited.get(device_name) {
                Some(true) => continue,
                _ => {
                    let mut component = Neighborhood::new();
                    self.bfs_explore(
                        &marked_nodes,
                        device,
                        &mut visited,
                        &mut component,
                        proximity_depth,
                        None,
                    );
                    components.push(component);
                }
            }
        }
        components
    }

    fn bfs_explore(
        &self,
        marked_nodes: &HashMap<String, PacketSpaceAwareDevice>,
        start_device: &PacketSpaceAwareDevice,
        visited: &mut HashMap<String, bool>,
        component: &mut Neighborhood,
        proximity_depth: i32,
        max_neighborhood_size: Option<usize>,
    ) {
        let mut queue: Vec<(String, i32)> = Vec::new();
        queue.push((start_device.device_name.clone(), proximity_depth));
        let mut marked_count = 0;
        while let Some((current_device_name, depth)) = queue.pop() {
            if *visited.get(&current_device_name).unwrap_or(&false) || depth == 0 {
                continue;
            }
            visited.insert(current_device_name.clone(), true);
            if let Some(bdd) = marked_nodes.get(&current_device_name) {
                component.add_marked_node(bdd.clone());
            } else {
                component.add_normal_node(current_device_name.clone());
            }
            if let Some(max_size) = max_neighborhood_size {
                if marked_nodes.contains_key(&current_device_name) {
                    marked_count += 1;
                    if marked_count >= max_size {
                        break;
                    }
                }
            }
            if let Some(cur_ports) = self
                .src_toponet
                .network
                .get_device_ports()
                .get(&current_device_name)
            {
                for cur_port in cur_ports {
                    if let Some((neighbor_name, _)) = cur_port.get_peer_port() {
                        if !*visited.get(neighbor_name).unwrap_or(&false) {
                            let new_depth = if marked_nodes.contains_key(neighbor_name) {
                                proximity_depth
                            } else {
                                depth - 1
                            };
                            queue.push((neighbor_name.clone(), new_depth));
                        }
                    }
                }
            }
        }
    }

    pub fn verify_reachability_with_npnet(&self) {
        let start = Instant::now();
        let neighborhoods = self.find_neighborhood_from_subnet_space();
        neighborhoods.par_iter().for_each(|neighborhood| {
            let mut npnet = NPNet::new_with_src_toponet(&self.src_toponet, neighborhood.clone());
            npnet.iterative_traversal(TraversalType::Backward, InvariantType::Reachability);
            npnet.check_reachability(self.edge_devices.clone(), TraversalType::Backward);
        });
        let duration = start.elapsed();
        println!("Verification time: {:?}", duration);
    }
}
