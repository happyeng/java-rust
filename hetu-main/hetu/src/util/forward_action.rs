use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct ForwardAction {
    forward_type: String,
    ports: Vec<String>,
}

impl ForwardAction {
    pub fn new(forward_type: String, ports: Vec<String>) -> Self {
        ForwardAction {
            forward_type,
            ports: ports,
        }
    }

    pub fn print_ports(&self) {
        for port in &self.ports {
            println!("{}", port);
        }
    }

    pub fn get_forward_type(&self) -> &String {
        &self.forward_type
    }

    pub fn get_ports(&self) -> &Vec<String> {
        &self.ports
    }
}

impl Hash for ForwardAction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.forward_type.hash(state);
        self.ports.hash(state);
    }
}

impl PartialEq for ForwardAction {
    fn eq(&self, other: &Self) -> bool {
        self.forward_type == other.forward_type && self.ports == other.ports
    }
}

impl Eq for ForwardAction {}

impl fmt::Display for ForwardAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ForwardAction {{ forward_type: {}, ports: {:?} }}",
            self.forward_type, self.ports
        )
    }
}
