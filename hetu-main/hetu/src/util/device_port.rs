use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct DevicePort {
    device_name: String,
    port_name: String,
    peer_key: Option<(String, String)>,
}

impl PartialEq for DevicePort {
    fn eq(&self, other: &Self) -> bool {
        self.device_name == other.device_name && self.port_name == other.port_name
    }
}

impl Eq for DevicePort {}

impl Hash for DevicePort {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.device_name.hash(state);
        self.port_name.hash(state);
    }
}

impl DevicePort {
    pub fn new(device_name: String, port_name: String) -> DevicePort {
        DevicePort {
            device_name,
            port_name,
            peer_key: None,
        }
    }

    pub fn get_device_name(&self) -> String {
        self.device_name.to_string()
    }

    pub fn get_port_name(&self) -> String {
        self.port_name.to_string()
    }

    pub fn set_peer_key(&mut self, peer_device_name: String, peer_port_name: String) {
        self.peer_key = Some((peer_device_name, peer_port_name));
    }

    pub fn get_peer_port(&self) -> Option<&(String, String)> {
        self.peer_key.as_ref()
    }
}
