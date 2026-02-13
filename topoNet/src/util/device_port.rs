#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DevicePort {
    device_name: String,
    port_name: String,
}

impl DevicePort {
    pub fn new(device_name: String, port_name: String) -> DevicePort {
        DevicePort {
            device_name,
            port_name,
        }
    }

    pub fn get_device_name(&self) -> String {
        self.device_name.to_string()
    }

    pub fn get_port_name(&self) -> String {
        self.port_name.to_string()
    }
}
