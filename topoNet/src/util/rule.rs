use crate::util::forward_action::ForwardAction;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct Rule {
    forward_action: ForwardAction,
    prefix_len: usize,
    ip: String,
}

impl Rule {
    pub fn new(prefix_len: usize, ip: String, forward_type: String, ports: Vec<String>) -> Self {
        let forward_action: ForwardAction = ForwardAction::new(forward_type, ports);
        Rule {
            forward_action: forward_action,
            prefix_len: prefix_len,
            ip: ip,
        }
    }

    pub fn new_for_packet_space(prefix_len: usize, ip: String) -> Self {
        let forward_action: ForwardAction =
            ForwardAction::new("packet_space".to_string(), Vec::new());
        Rule {
            forward_action: forward_action,
            prefix_len: prefix_len,
            ip: ip,
        }
    }

    pub fn get_prefix_len(&self) -> usize {
        self.prefix_len
    }

    pub fn get_ip(&self) -> &str {
        &self.ip
    }

    pub fn get_forward_action(&self) -> ForwardAction {
        self.forward_action.clone()
    }
}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the fields that contribute to the equality and uniqueness of the Rule
        self.forward_action.hash(state);
        self.prefix_len.hash(state);
        self.ip.hash(state);
    }
}

impl Eq for Rule {}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        // Implement custom equality comparison based on the fields
        self.forward_action == other.forward_action
            && self.prefix_len == other.prefix_len
            && self.ip == other.ip
    }
}
