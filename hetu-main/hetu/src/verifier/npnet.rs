use super::{device::Device, toponet::Toponet};
use crate::util::hash_utils::{HashMap, HashSet};
use crate::util::npbdd::NPBDD;
use crate::util::{device_port::DevicePort, network::Network};
use crate::verifier::neighborhood::Neighborhood;
use crate::{EXIST_COUNT, NONEXIST_COUNT, TRAVERSAL_COUNT};
use biodivine_lib_bdd::*;
use std::sync::atomic::Ordering;
use std::sync::Arc;

struct Tunnel {
    pub device_port: DevicePort,
    pub arrive_spaces: Vec<Bdd>,
    pub veriyied_space: Bdd,
    pub arrive_cnt: u32,
}

impl Tunnel {
    pub fn verified_space_prune(&mut self, predicate: &Bdd) -> Bdd {
        let extra_space = predicate.and_not(&self.veriyied_space);
        if extra_space.is_false() {
            return extra_space;
        }
        self.veriyied_space = self.veriyied_space.or(&extra_space);
        extra_space
    }

    pub fn trans_space_to_peer_node(
        &mut self,
        inner_area: &mut HashMap<String, NPNetNode>,
        outer_area: &mut HashMap<String, NPNetNode>,
    ) {
        let aggre_space = self
            .arrive_spaces
            .iter()
            .fold(NPBDD::make_none_space_bdd(), |acc, space| acc.or(space));
        self.arrive_spaces.clear();
        let dst_node_name = self.device_port.get_peer_port().unwrap().0.clone();
        if inner_area.contains_key(&dst_node_name) {
            let dst_node = inner_area.get_mut(&dst_node_name).unwrap();
            dst_node.arrive_spaces.push(aggre_space.clone());
            dst_node.veriyied_space = dst_node.veriyied_space.or(&aggre_space);
        } else if outer_area.contains_key(&dst_node_name) {
            let dst_node = outer_area.get_mut(&dst_node_name).unwrap();
            dst_node.arrive_spaces.push(aggre_space.clone());
            dst_node.veriyied_space = dst_node.veriyied_space.or(&aggre_space);
        }
    }

    pub fn loop_detection_at_port(&mut self, max_arrive_cnt: u32) -> bool {
        self.arrive_cnt += 1;
        if self.arrive_cnt > max_arrive_cnt {
            println!(
                "Possible loop detected at port {} for device {}",
                self.device_port.get_port_name(),
                self.device_port.get_peer_port().unwrap().0
            );
            return true;
        }
        false
    }
}

struct NPNetNode {
    pub name: String,
    device: Arc<Device>,
    arrive_spaces: Vec<Bdd>,
    veriyied_space: Bdd,
    pub port_arrive_cnt: HashMap<String, u32>,
}

impl NPNetNode {
    pub fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            device: Arc::new(Device::new(name)),
            arrive_spaces: Vec::new(),
            veriyied_space: NPBDD::make_none_space_bdd(),
            port_arrive_cnt: HashMap::default(),
        }
    }

    pub fn get_forwardable_space(&self) -> Bdd {
        self.device.forwardable_space.clone()
    }

    pub fn verified_space_prune(&mut self, predicate: &Bdd) -> Bdd {
        let extra_space = predicate.and_not(&self.veriyied_space);
        if extra_space.is_false() {
            return extra_space;
        }
        self.veriyied_space = self.veriyied_space.or(&extra_space);
        extra_space
    }
    pub fn arrive_space_aggregate_and_verify(&mut self) -> Bdd {
        let aggre_sapce = self
            .arrive_spaces
            .iter()
            .fold(NPBDD::make_none_space_bdd(), |acc, space| acc.or(space));
        self.veriyied_space = self.veriyied_space.or(&aggre_sapce);
        self.arrive_spaces.clear();
        aggre_sapce
    }

    pub fn loop_detection_at_port(&mut self, port_name: String, max_arrive_cnt: u32) -> bool {
        match self.port_arrive_cnt.get_mut(&port_name) {
            Some(cnt) => {
                *cnt += 1;
                if *cnt > max_arrive_cnt {
                    println!(
                        "Possible loop detected at port {} for device {}",
                        port_name, self.name
                    );
                    return true;
                }
                false
            }
            None => {
                self.port_arrive_cnt.insert(port_name, 1);
                false
            }
        }
    }
}

struct NPNetCtx {
    pub device_name: String,
    pub arrive_predicate: Bdd,
}

pub struct NPNet {
    neighborhood: Neighborhood,
    pub inner_area: HashMap<String, NPNetNode>,
    pub outer_area: HashMap<String, NPNetNode>,
    pub entrance: HashMap<DevicePort, Tunnel>,
    devices: Arc<HashMap<String, Arc<Device>>>,
    network: Arc<Network>,
    all_subnet_space: Bdd,
    map_device_packet_space_bdd: Arc<HashMap<String, Bdd>>,
}

impl NPNet {
    pub fn new_with_src_toponet(src_toponet: &Toponet, neighborhood: Neighborhood) -> Self {
        let bdd = src_toponet.all_space_map.get("All").unwrap().clone();
        let mut npnet = Self::gen_npnet(
            neighborhood,
            src_toponet.devices.clone(),
            src_toponet.network.clone(),
            bdd,
            src_toponet.map_device_packet_space_bdd.clone(),
        );
        let nodes = npnet.gen_nodes();
        npnet.partion_npnet(nodes);
        npnet
    }

    fn gen_npnet(
        neighborhood: Neighborhood,
        devices: Arc<HashMap<String, Arc<Device>>>,
        network: Arc<Network>,
        all_subnet_space: Bdd,
        map_device_packet_space_bdd: Arc<HashMap<String, Bdd>>,
    ) -> Self {
        Self {
            neighborhood,
            inner_area: HashMap::default(),
            outer_area: HashMap::default(),
            entrance: HashMap::default(),
            devices,
            network,
            all_subnet_space,
            map_device_packet_space_bdd,
        }
    }

    fn gen_nodes(&self) -> Vec<NPNetNode> {
        let mut nodes = Vec::new();
        for (device_name, device) in self.devices.iter() {
            let mut node = NPNetNode::new(device_name.clone().into());
            node.device = Arc::clone(device);
            nodes.push(node);
        }
        nodes
    }

    fn partion_npnet(&mut self, nodes: Vec<NPNetNode>) {
        for node in nodes {
            let device_name = node.name.as_ref();
            if self.neighborhood.is_neighborhood_node(device_name) {
                self.inner_area.insert(device_name.to_string(), node);
            } else {
                self.outer_area.insert(device_name.to_string(), node);
            }
        }
        for (node_name, is_inner) in self
            .inner_area
            .keys()
            .map(|n| (n, true))
            .chain(self.outer_area.keys().map(|n| (n, false)))
        {
            let ports = self.network.get_device_ports().get(node_name);
            if ports.is_none() {
                continue;
            }
            for device_port in ports.unwrap() {
                let (peer_name, _) = device_port.get_peer_port().unwrap();
                if (is_inner && !self.inner_area.contains_key(peer_name))
                    || (!is_inner && self.inner_area.contains_key(peer_name))
                {
                    let tunnel = Tunnel {
                        device_port: device_port.clone(),
                        arrive_spaces: Vec::new(),
                        veriyied_space: NPBDD::make_none_space_bdd(),
                        arrive_cnt: 0,
                    };
                    self.entrance.insert(device_port.clone(), tunnel);
                }
            }
        }
    }
}

#[derive(Clone)]
pub enum TraversalType {
    Forward,
    Backward,
}

#[derive(Clone)]
pub enum InvariantType {
    Reachability,
}

impl NPNet {
    pub fn iterative_traversal(
        &mut self,
        traversal_type: TraversalType,
        invariant_type: InvariantType,
    ) {
        self.init_marked_nodes_packet_space(traversal_type.clone());
        let mut iteration = 0;
        loop {
            iteration += 1;
            self.traverse_inner_area(traversal_type.clone(), invariant_type.clone());
            if self.entrace_check() {
                break;
            }
            self.traverse_outer_area(traversal_type.clone(), invariant_type.clone());
            if self.entrace_check() {
                break;
            }
        }
    }

    fn init_marked_nodes_packet_space(&mut self, traversal_type: TraversalType) {
        let marked_nodes = self.neighborhood.get_marked_nodes();
        for (name, device) in marked_nodes {
            let start_node = self.inner_area.get_mut(name).unwrap();
            let device_bdd = NPBDD::make_src_device_bdd(device.device_id);
            let packet_space = match traversal_type {
                TraversalType::Forward => device_bdd.and(&self.all_subnet_space),
                TraversalType::Backward => device.dst_prefix_bdd.clone(),
            };
            start_node.arrive_spaces.push(packet_space);
        }
    }

    fn traverse_inner_area(
        &mut self,
        traversal_type: TraversalType,
        invariant_type: InvariantType,
    ) {
        let mut queue = Vec::new();
        let mut traversal_count = 0;
        for (node_name, node) in &mut self.inner_area {
            if node.arrive_spaces.is_empty() {
                continue;
            }
            queue.push(NPNetCtx {
                device_name: node_name.clone(),
                arrive_predicate: node.arrive_space_aggregate_and_verify(),
            });
        }

        while !queue.is_empty() {
            let mut indegree_check_set: HashSet<String> = HashSet::default();
            let size = queue.len();
            for _ in 0..size {
                let current_ctx = queue.remove(0);
                let current_node_name = current_ctx.device_name;
                let arrive_predicate = current_ctx.arrive_predicate;
                if let Some(ports) = self.network.get_device_ports().get(&current_node_name) {
                    for port in ports {
                        if let Some((dst_device_name, dst_port_name)) = port.get_peer_port() {
                            traversal_count += 1;
                            let dst_device = self.devices.get(dst_device_name).unwrap();
                            let mut intersection = NPBDD::make_none_space_bdd();
                            match traversal_type {
                                TraversalType::Forward => {
                                    let current_device =
                                        self.devices.get(&current_node_name).unwrap();
                                    if current_device.has_space_bdd(&port.get_port_name()) {
                                        intersection = arrive_predicate.and(
                                            &current_device.get_space_bdd(&port.get_port_name()),
                                        );
                                    } else {
                                        continue;
                                    }
                                }
                                TraversalType::Backward => {
                                    if dst_device.has_space_bdd(dst_port_name) {
                                        intersection = arrive_predicate
                                            .and(&dst_device.get_space_bdd(dst_port_name));
                                    } else {
                                        continue;
                                    }
                                }
                            }
                            if intersection.is_false() {
                                continue;
                            }
                            if self.inner_area.contains_key(dst_device_name) {
                                let dst_node = self.inner_area.get_mut(dst_device_name).unwrap();
                                let mut arrive_space = intersection.clone();
                                match invariant_type {
                                    _ => {
                                        arrive_space = dst_node.verified_space_prune(&arrive_space);
                                    }
                                }
                                if !arrive_space.is_false() {
                                    indegree_check_set.insert(dst_device_name.clone());
                                    dst_node.arrive_spaces.push(arrive_space);
                                }
                            }
                            if self.entrance.contains_key(port) {
                                if let Some(tunnel) = self.entrance.get_mut(port) {
                                    let mut arrive_space = intersection;
                                    match invariant_type {
                                        _ => {
                                            arrive_space =
                                                tunnel.verified_space_prune(&arrive_space);
                                        }
                                    }
                                    if !arrive_space.is_false() {
                                        tunnel.arrive_spaces.push(arrive_space);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            indegree_check_set.iter().for_each(|node_name| {
                let node = self.inner_area.get_mut(node_name).unwrap();
                let aggreated_space = node.arrive_space_aggregate_and_verify();
                queue.push(NPNetCtx {
                    device_name: node_name.clone(),
                    arrive_predicate: aggreated_space,
                });
            });
        }
        TRAVERSAL_COUNT.fetch_add(traversal_count, Ordering::SeqCst);
    }

    fn entrace_check(&mut self) -> bool {
        let mut space_convergence = true;
        for tunnel in self.entrance.values_mut() {
            if tunnel.arrive_spaces.is_empty() {
                continue;
            } else {
                space_convergence = false;
                tunnel.trans_space_to_peer_node(&mut self.inner_area, &mut self.outer_area);
            }
        }
        space_convergence
    }

    fn traverse_outer_area(
        &mut self,
        traversal_type: TraversalType,
        invariant_type: InvariantType,
    ) {
        let mut queue = Vec::new();
        let mut traversal_count = 0;
        for (node_name, node) in &mut self.outer_area {
            if node.arrive_spaces.is_empty() {
                continue;
            }
            queue.push(NPNetCtx {
                device_name: node_name.clone(),
                arrive_predicate: node.arrive_space_aggregate_and_verify(),
            });
        }

        while !queue.is_empty() {
            let mut indegree_check_set: HashSet<String> = HashSet::default();
            let size = queue.len();
            for _ in 0..size {
                let current_ctx = queue.remove(0);
                let current_node_name = current_ctx.device_name;
                let arrive_predicate = current_ctx.arrive_predicate;
                if let Some(ports) = self.network.get_device_ports().get(&current_node_name) {
                    for port in ports {
                        if let Some((dst_device_name, dst_port_name)) = port.get_peer_port() {
                            traversal_count += 1;
                            let dst_device = self.devices.get(dst_device_name).unwrap();
                            let mut intersection = NPBDD::make_none_space_bdd();
                            match traversal_type {
                                TraversalType::Forward => {
                                    let current_device =
                                        self.devices.get(&current_node_name).unwrap();
                                    if current_device.has_space_bdd(&port.get_port_name()) {
                                        intersection = arrive_predicate.and(
                                            &current_device.get_space_bdd(&port.get_port_name()),
                                        );
                                    } else {
                                        continue;
                                    }
                                }
                                TraversalType::Backward => {
                                    if dst_device.has_space_bdd(dst_port_name) {
                                        intersection = arrive_predicate
                                            .and(&dst_device.get_space_bdd(dst_port_name));
                                    } else {
                                        continue;
                                    }
                                }
                            }
                            if intersection.is_false() {
                                continue;
                            }
                            if self.outer_area.contains_key(dst_device_name) {
                                let dst_node = self.outer_area.get_mut(dst_device_name).unwrap();
                                let mut arrive_space = intersection.clone();
                                match invariant_type {
                                    _ => {
                                        arrive_space = dst_node.verified_space_prune(&arrive_space);
                                    }
                                }
                                if !arrive_space.is_false() {
                                    indegree_check_set.insert(dst_device_name.clone());
                                    dst_node.arrive_spaces.push(arrive_space);
                                }
                            }
                            if self.entrance.contains_key(port) {
                                if let Some(tunnel) = self.entrance.get_mut(port) {
                                    let mut arrive_space = intersection;
                                    match invariant_type {
                                        _ => {
                                            arrive_space =
                                                tunnel.verified_space_prune(&arrive_space);
                                        }
                                    }
                                    if !arrive_space.is_false() {
                                        tunnel.arrive_spaces.push(arrive_space);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            indegree_check_set.iter().for_each(|node_name| {
                let node = self.outer_area.get_mut(node_name).unwrap();
                let aggreated_space = node.arrive_space_aggregate_and_verify();
                queue.push(NPNetCtx {
                    device_name: node_name.clone(),
                    arrive_predicate: aggreated_space,
                });
            });
        }
        TRAVERSAL_COUNT.fetch_add(traversal_count, Ordering::SeqCst);
    }
}

impl NPNet {
    pub fn check_reachability(&self, pair_devices: HashSet<String>, traversal_type: TraversalType) {
        match traversal_type {
            TraversalType::Backward => self.backward_check_reachability(pair_devices),
            TraversalType::Forward => self.forward_check_reachability(pair_devices),
        }
    }

    pub fn backward_check_reachability(&self, pair_devices: HashSet<String>) {
        let mut arrive_count = 0;
        let mut unreachable_count = 0;
        for src_name in pair_devices {
            let src_node;
            match self.inner_area.get(&src_name) {
                Some(node) => src_node = node,
                None => src_node = self.outer_area.get(&src_name).unwrap(),
            }
            let dst_nodes = self.neighborhood.get_marked_nodes();
            for (dst_node_name, device) in dst_nodes {
                if *dst_node_name == src_name {
                    continue;
                }
                if device
                    .dst_prefix_bdd
                    .and_not(&src_node.veriyied_space)
                    .is_false()
                {
                    arrive_count += 1;
                } else {
                    unreachable_count += 1;
                }
            }
        }
        EXIST_COUNT.fetch_add(arrive_count, Ordering::SeqCst);
        NONEXIST_COUNT.fetch_add(unreachable_count, Ordering::SeqCst);
    }

    pub fn forward_check_reachability(&self, pair_devices: HashSet<String>) {
        let mut reach_cnt = 0;
        let mut unreach_cnt = 0;
        let src_nodes = self.neighborhood.get_marked_nodes();
        let dst_nodes: Vec<&NPNetNode> = pair_devices
            .iter()
            .map(|device_name| match self.inner_area.get(device_name) {
                Some(node) => node,
                None => self.outer_area.get(device_name).unwrap(),
            })
            .collect();

        dst_nodes.iter().for_each(|dst_node| {
            let dst_node_name = dst_node.name.clone();
            let Some(dst_node_subnet_space) = self.map_device_packet_space_bdd.get(&dst_node_name)
            else {
                return;
            };
            src_nodes.iter().for_each(|(src_node_name, src_node)| {
                if dst_node_name == *src_node_name {
                    return;
                }
                let src_device_space = NPBDD::make_src_device_bdd(src_node.device_id);
                let packet_space = src_device_space.and(dst_node_subnet_space);
                if !packet_space.and_not(&dst_node.veriyied_space).is_false() {
                    unreach_cnt += 1;
                } else {
                    reach_cnt += 1;
                }
            });
        });
        NONEXIST_COUNT.fetch_add(unreach_cnt, Ordering::SeqCst);
        EXIST_COUNT.fetch_add(reach_cnt, Ordering::SeqCst);
    }
}
