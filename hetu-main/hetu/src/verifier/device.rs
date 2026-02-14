use super::lec::Lec;
use crate::simulator::SubNet;
use crate::util::hash_utils::{HashMap, HashSet};
use crate::util::npbdd::NPBDD;
use crate::util::{rule::Rule, space_port::SpacePort};
use biodivine_lib_bdd::Bdd;
use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

lazy_static! {
    pub static ref ALIVE_DEVICE_MAP: DashMap<String, DashSet<i16>> = DashMap::new();
}

#[derive(Clone)]
pub struct Device {
    name: String,
    pub device_id: usize,
    pub device_id_bdd: Bdd,
    rules: Vec<Rule>,
    packet_space: Option<Rule>,
    port_name_to_space_id: HashMap<String, i8>,
    space_id_to_space_port: HashMap<i8, SpacePort>,
    pub forwardable_space: Bdd,
    pub subnet_space: Bdd,
}

#[derive(Serialize, Deserialize)]
struct Record {
    #[serde(alias = "forward_type")]
    action: String,
    #[serde(alias = "ip")]
    prefix: String,
    #[serde(alias = "ports")]
    nexthop_infs: Vec<String>,
    prefix_len: usize,
}

impl Device {
    pub fn new(name: String) -> Self {
        Device {
            name,
            device_id: 0,
            device_id_bdd: NPBDD::make_none_space_bdd(),
            rules: Vec::new(),
            packet_space: None,
            port_name_to_space_id: HashMap::default(),
            space_id_to_space_port: HashMap::default(),
            forwardable_space: NPBDD::make_none_space_bdd(),
            subnet_space: NPBDD::make_none_space_bdd(),
        }
    }

    pub fn read_rules_file(&mut self, filename: &String) {
        let contents = fs::read_to_string(filename).expect("Error while reading the file");

        let records: Vec<Record> =
            serde_json::from_str(&contents).expect("Error while parsing the JSON {}");

        let record_count = records.len();

        self.rules.reserve(record_count);

        for record in records {
            let rule = Rule::new(
                record.prefix_len,
                record.prefix,
                record.action,
                record.nexthop_infs,
            );
            self.rules.push(rule);
        }

        self.rules
            .sort_by_key(|rule| std::cmp::Reverse(rule.get_prefix_len()));
    }

    pub fn merge_lec_to_space_port(&mut self, tmp_lecs: HashSet<Lec>) {
        let mut tmp_space_port: HashMap<Bdd, i8> = HashMap::default();
        let mut cur_space_id = 0;
        ALIVE_DEVICE_MAP.insert(self.name.clone(), DashSet::new());
        for lec in tmp_lecs {
            if let Some(port_name) = lec.forward_action.get_ports().first() {
                if let Some(&tmp_space_id) = tmp_space_port.get(&lec.predicate) {
                    self.port_name_to_space_id
                        .insert(port_name.to_string(), tmp_space_id);
                } else {
                    self.space_id_to_space_port.insert(
                        cur_space_id,
                        SpacePort::new(cur_space_id, lec.predicate.clone()),
                    );
                    if let Some(mut device_set) = ALIVE_DEVICE_MAP.get_mut(&self.name) {
                        device_set.insert(cur_space_id as i16);
                    }
                    self.port_name_to_space_id
                        .insert(port_name.to_string(), cur_space_id);
                    tmp_space_port.insert(lec.predicate, cur_space_id);
                    cur_space_id += 1;
                }
            } else {
                eprintln!("Ports list is empty");
                return;
            }
        }
    }

    pub fn cal_forwardable_space(&mut self, tmp_lecs: &HashSet<Lec>) {
        for lec in tmp_lecs {
            self.forwardable_space = self.forwardable_space.or(&lec.predicate);
        }
    }

    pub fn check_intersection_at_port(&self, arrive_bdd: &Bdd, port_name: &str) -> bool {
        match self.has_space_bdd(port_name) {
            false => false,
            true => {
                let port_space_bdd = self.get_space_bdd(port_name);
                arrive_bdd.and_not(port_space_bdd).is_false()
            }
        }
    }

    pub fn routes_table_prefix_match(&self, subnet: &SubNet) -> Option<Rule> {
        let rules = self.get_rules();
        for rule in rules {
            if Self::match_check(rule, subnet) {
                println!("rule found")
            }
        }
        None
    }

    fn match_check(rule: &Rule, subnet: &SubNet) -> bool {
        let subnet_network = subnet.prefix;
        let subnet_prefix_len = subnet.prefix_len;

        let rule_network: IpAddr = rule.get_ip().parse().unwrap();
        let rule_prefix_len = rule.get_prefix_len() as u8;

        if subnet_prefix_len < rule_prefix_len {
            return false;
        }
        let subnet_masked = Self::apply_mask(subnet_network, rule_prefix_len);
        let rule_masked = Self::apply_mask(rule_network, rule_prefix_len);
        subnet_masked == rule_masked
    }

    fn apply_mask(ip: IpAddr, prefix_len: u8) -> IpAddr {
        match ip {
            IpAddr::V4(addr) => {
                let mask = u32::MAX << (32 - prefix_len as u32);
                IpAddr::V4(Ipv4Addr::from(u32::from(addr) & mask))
            }
            IpAddr::V6(addr) => {
                let mask = u128::MAX << (128 - prefix_len as u32);
                IpAddr::V6(Ipv6Addr::from(u128::from(addr) & mask))
            }
        }
    }

    pub fn get_space_bdd(&self, port_name: &str) -> &Bdd {
        let cur_id = self.port_name_to_space_id.get(port_name).unwrap();
        let cur_space_port = self.space_id_to_space_port.get(cur_id).unwrap();
        cur_space_port.get_space()
    }

    pub fn get_space_id(&self, port_name: &String) -> i8 {
        let cur_id = self.port_name_to_space_id.get(port_name).unwrap();
        let cur_space_port = self.space_id_to_space_port.get(cur_id).unwrap();
        cur_space_port.get_space_id()
    }

    pub fn get_space_id_to_space_port(&self) -> &HashMap<i8, SpacePort> {
        &self.space_id_to_space_port
    }

    pub fn has_space_bdd(&self, port_name: &str) -> bool {
        self.port_name_to_space_id.contains_key(port_name)
    }

    pub fn set_packet_space_file(&mut self, packet_space: Rule) {
        self.packet_space = Some(packet_space);
    }

    pub fn get_packet_space(&self) -> &Option<Rule> {
        &self.packet_space
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_rules(&self) -> &Vec<Rule> {
        &self.rules
    }

    pub fn take_rules(&mut self) -> Vec<Rule> {
        std::mem::take(&mut self.rules)
    }

    pub fn get_rules_mut(&mut self) -> &mut Vec<Rule> {
        &mut self.rules
    }
}
