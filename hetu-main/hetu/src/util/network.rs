use super::pod::Pod;
use crate::util::device_port::DevicePort;
use crate::util::hash_utils::{HashMap, HashSet};
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Topology {
    pub dst_node: String,
    pub dst_port: String,
    pub src_node: String,
    pub src_port: String,
}

pub struct Network {
    device_ports: HashMap<String, HashSet<DevicePort>>,
    topology: HashMap<DevicePort, DevicePort>,
    pods: HashMap<i32, Pod>,
    pod_device_names: HashSet<String>,
}

impl Network {
    pub fn new() -> Network {
        Network {
            device_ports: HashMap::default(),
            topology: HashMap::default(),
            pods: HashMap::default(),
            pod_device_names: HashSet::default(),
        }
    }

    pub fn read_topology_by_file(&mut self, filepath: &str) {
        let file = File::open(filepath).expect("Error opening the file");
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .expect("Error reading the file");
        let topologies: Vec<Topology> =
            serde_json::from_str(&content).expect("Error parsing the JSON");

        let tmp_device_ports: DashMap<String, HashSet<DevicePort>> = DashMap::default();
        topologies.par_iter().for_each(|topology| {
            let d1 = topology.src_node.clone();
            let p1 = topology.src_port.clone();
            let d2 = topology.dst_node.clone();
            let p2 = topology.dst_port.clone();
            let mut dp1 = DevicePort::new(d1.clone(), p1.clone());
            let mut dp2 = DevicePort::new(d2.clone(), p2.clone());

            dp1.set_peer_key(d2.clone(), p2.clone());
            dp2.set_peer_key(d1.clone(), p1.clone());

            tmp_device_ports.entry(d1).or_default().insert(dp1.clone());
            tmp_device_ports.entry(d2).or_default().insert(dp2.clone());
        });

        self.device_ports = tmp_device_ports.into_iter().collect();
        let mut pods = self.find_pods(&topologies);
        pods.par_iter_mut().for_each(|(_, pod)| {
            pod.set_interfaces(&self.device_ports);
        });
        self.pods = pods;
    }

    pub fn find_pods(&mut self, topologies: &Vec<Topology>) -> HashMap<i32, Pod> {
        let mut pod_map: HashMap<i32, Pod> = HashMap::default();
        let mut graph: HashMap<String, Vec<String>> = HashMap::default();
        for topology in topologies {
            if topology.dst_node.contains("S1") && topology.src_node.contains("S0") {
                graph
                    .entry(topology.dst_node.clone())
                    .or_insert_with(Vec::new)
                    .push(topology.src_node.clone());
                graph
                    .entry(topology.src_node.clone())
                    .or_insert_with(Vec::new)
                    .push(topology.dst_node.clone());
            }
        }
        let mut visited: HashSet<String> = HashSet::default();
        let mut pod_id = 0;
        for node in graph.keys() {
            if !visited.contains(node) {
                pod_id += 1;
                let mut queue = VecDeque::new();
                queue.push_back(node.clone());
                let mut pod = Pod::new(pod_id);
                while let Some(current_node) = queue.pop_front() {
                    if !visited.insert(current_node.clone()) {
                        continue;
                    }
                    self.pod_device_names.insert(current_node.clone());

                    if current_node.contains("S1") {
                        pod.add_s1_device(current_node.clone());
                    } else if current_node.contains("S0") {
                        pod.add_s0_device(current_node.clone());
                    }
                    if let Some(neighbors) = graph.get(&current_node) {
                        for neighbor in neighbors {
                            if !visited.contains(neighbor) {
                                queue.push_back(neighbor.clone());
                            }
                        }
                    }
                }
                pod_map.insert(pod_id, pod);
            }
        }
        pod_map
    }

    pub fn get_pods(&self) -> &HashMap<i32, Pod> {
        &self.pods
    }

    pub fn get_pod_device_names(&self) -> &HashSet<String> {
        &self.pod_device_names
    }

    pub fn get_device_ports(&self) -> &HashMap<String, HashSet<DevicePort>> {
        &self.device_ports
    }

    pub fn get_toplogy(&self) -> &HashMap<DevicePort, DevicePort> {
        &self.topology
    }

    pub fn print_pod_device_names(&self) {
        for device_name in &self.pod_device_names {
            println!("{}", device_name);
        }
    }
}
