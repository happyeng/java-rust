use super::device_port::DevicePort;
use crate::util::hash_utils::{HashMap, HashSet};

#[derive(Debug)]
pub struct Pod {
    pub pod_id: i32,
    pub s1_device_names: HashSet<String>,
    pub s0_device_names: HashSet<String>,
    pub interfaces: HashSet<DevicePort>,
    pub has_dst_device: bool,
}

impl Pod {
    pub fn new(pod_id: i32) -> Self {
        Pod {
            pod_id,
            s1_device_names: HashSet::default(),
            s0_device_names: HashSet::default(),
            interfaces: HashSet::default(),
            has_dst_device: false,
        }
    }

    pub fn get_pod_id(&self) -> i32 {
        self.pod_id
    }

    pub fn get_s1_device_names(&self) -> &HashSet<String> {
        &self.s1_device_names
    }

    pub fn get_s0_device_names(&self) -> &HashSet<String> {
        &self.s0_device_names
    }

    pub fn add_s1_device(&mut self, device_name: String) {
        self.s1_device_names.insert(device_name);
    }

    pub fn add_s0_device(&mut self, device_name: String) {
        self.s0_device_names.insert(device_name);
    }

    pub fn set_interfaces(&mut self, device_ports: &HashMap<String, HashSet<DevicePort>>) {
        let mut ports_to_add: HashSet<DevicePort> = HashSet::default();
        for s1_device_name in &self.s1_device_names {
            if let Some(ports) = device_ports.get(s1_device_name) {
                for port in ports {
                    if let Some((peer_device_name, _peer_port_name)) = port.get_peer_port() {
                        if !self.s0_device_names.contains(peer_device_name) {
                            self.interfaces.insert(port.clone());
                            ports_to_add.insert(port.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn show_info(&self) {
        self.s1_device_names.iter().for_each(|device_name| {
            println!("Device Name: {}", device_name);
        });
        self.s0_device_names.iter().for_each(|device_name| {
            println!("Device Name: {}", device_name);
        });
    }

    pub fn get_devices(&self) -> Vec<String> {
        let mut devices = Vec::new();
        devices.extend(self.s1_device_names.iter().cloned());
        devices.extend(self.s0_device_names.iter().cloned());
        devices
    }
}
