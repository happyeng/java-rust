use crate::util::rule::Rule;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Clone)]
pub struct Device {
    name: String,
    rules: Vec<Rule>,
    packet_space: Option<Rule>,
}

#[derive(Serialize, Deserialize, Debug)]
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
            rules: Vec::new(),
            packet_space: None,
        }
    }

    pub fn read_rules_file(&mut self, filename: &String) {
        let contents = fs::read_to_string(filename).expect("Error while reading the file");
        let records: Vec<Record> = serde_json::from_str(&contents).unwrap_or_else(|err| {
            panic!("Error while parsing the JSON {}: {}", filename, err);
        });

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

    pub fn get_rules_mut(&mut self) -> &mut Vec<Rule> {
        &mut self.rules
    }
}
