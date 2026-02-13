use biodivine_lib_bdd::*;
use std::net::IpAddr;

#[allow(unused)]
pub struct BddEngine {
    ip_bits_len: usize,
    variables_dst_ip: Vec<BddVariable>,
    variable_set: BddVariableSet,
}

impl BddEngine {
    pub fn new(ip_bits: usize) -> Self {
        let mut variable_builder = BddVariableSetBuilder::new();
        let mut variables_dst_ip = Vec::new();

        for i in 0..ip_bits {
            let var_name: String = format!("x{}", i + 1);
            let var = variable_builder.make_variable(&var_name);
            variables_dst_ip.push(var);
        }

        let variable_set = variable_builder.build();

        BddEngine {
            ip_bits_len: ip_bits,
            variables_dst_ip,
            variable_set,
        }
    }

    pub fn encode_dst_ip_prefix(
        ip_address: &str,
        prefix_length: usize,
        variables: &[BddVariable],
        variable_set: &BddVariableSet,
        ip_bits: usize,
    ) -> Bdd {
        if prefix_length == 0 {
            return variable_set.mk_true();
        }

        let ip_addr: IpAddr = ip_address.parse().unwrap();
        let octets = match ip_addr {
            IpAddr::V4(ipv4) => {
                let octets_v4 = ipv4
                    .octets()
                    .iter()
                    .map(|&x| x as u32)
                    .collect::<Vec<u32>>();
                octets_v4
            }
            IpAddr::V6(ipv6) => {
                let mut octets_v6 = [0u16; 8];
                let ipv6_octets = ipv6.octets();
                for i in 0..8 {
                    octets_v6[i] =
                        u16::from_be_bytes(ipv6_octets[2 * i..2 * i + 2].try_into().unwrap());
                }
                let octets_v6 = octets_v6.iter().map(|&x| x as u32).collect::<Vec<u32>>();
                octets_v6
            }
        };

        let oct_len = octets.len() * 2;
        let mut bdd = variable_set.mk_true();
        for (i, &var) in variables.iter().enumerate() {
            if i < (ip_bits - prefix_length) {
            } else {
                let mask = 1 << (i % oct_len);
                let octet_index = (ip_bits - 1 - i) / oct_len;
                let octet_value = octets[octet_index];
                if (octet_value & mask) != 0 {
                    bdd = bdd.and(&variable_set.mk_var(var));
                } else {
                    bdd = bdd.and(&variable_set.mk_not_var(var));
                }
            }
        }
        bdd
    }
}
