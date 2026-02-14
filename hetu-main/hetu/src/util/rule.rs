use serde::{Deserialize, Serialize};

use crate::util::forward_action::ForwardAction;
use std::hash::{Hash, Hasher};
#[derive(Clone, Debug)]
pub struct Rule {
    forward_action: ForwardAction,
    prefix_len: usize,
    ip: String,
}

#[derive(Serialize, Deserialize)]
struct Record {
    action: String,
    prefix: String,
    nexthop_infs: Vec<String>,
    prefix_len: usize,
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

    pub fn get_name(&self) -> String {
        format!("{}/{}", self.ip, &self.prefix_len)
    }

    pub fn get_forward_action(&self) -> ForwardAction {
        self.forward_action.clone()
    }
}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.forward_action.hash(state);
        self.prefix_len.hash(state);
        self.ip.hash(state);
    }
}

impl Eq for Rule {}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.forward_action == other.forward_action
            && self.prefix_len == other.prefix_len
            && self.ip == other.ip
    }
}

impl From<Record> for Rule {
    fn from(record: Record) -> Self {
        Rule {
            forward_action: ForwardAction::new(record.action, record.nexthop_infs),
            prefix_len: record.prefix_len,
            ip: record.prefix,
        }
    }
}
