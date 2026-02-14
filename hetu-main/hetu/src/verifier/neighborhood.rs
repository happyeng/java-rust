use crate::util::hash_utils::{HashMap, HashSet};
use biodivine_lib_bdd::Bdd;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PacketSpaceAwareDevice {
    pub device_name: String,
    pub dst_prefix_bdd: Bdd,
    pub device_id: usize,
}

impl PacketSpaceAwareDevice {
    pub fn new(device_name: String, dst_prefix_bdd: Bdd, device_id: usize) -> Self {
        Self {
            device_name,
            dst_prefix_bdd,
            device_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighborhood {
    local_marked_nodes: HashMap<String, PacketSpaceAwareDevice>,
    local_normal_nodes: HashSet<String>,
}

impl Neighborhood {
    pub fn new() -> Self {
        Self {
            local_marked_nodes: HashMap::default(),
            local_normal_nodes: HashSet::default(),
        }
    }

    pub fn add_marked_node(&mut self, packet_space_aware_device: PacketSpaceAwareDevice) {
        self.local_marked_nodes.insert(
            packet_space_aware_device.device_name.clone(),
            packet_space_aware_device,
        );
    }

    pub fn add_normal_node(&mut self, node: String) {
        self.local_normal_nodes.insert(node);
    }

    pub fn is_neighborhood_node(&self, device_name: &str) -> bool {
        self.local_marked_nodes.contains_key(device_name)
            || self.local_normal_nodes.contains(device_name)
    }

    pub fn is_marked_node(&self, device_name: &str) -> bool {
        self.local_marked_nodes.contains_key(device_name)
    }

    pub fn is_normal_node(&self, device_name: &str) -> bool {
        self.local_normal_nodes.contains(device_name)
    }

    pub fn get_marked_nodes(&self) -> &HashMap<String, PacketSpaceAwareDevice> {
        &self.local_marked_nodes
    }
}
