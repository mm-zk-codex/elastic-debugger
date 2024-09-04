use alloy::primitives::{address, Address, U256};

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

// Define the global singleton map using lazy_static and RwLock for safe concurrent access
lazy_static! {
    static ref ADDRESS_MAPPING: RwLock<HashMap<Address, String>> = {
        let initial_entries = vec![
            (
                address!("0000000000000000000000000000000000008006"),
                "Deployer",
            ),
            (
                address!("0000000000000000000000000000000000010002"),
                "Bridgehub",
            ),
            (
                address!("0000000000000000000000000000000000010003"),
                "Shared Bridge",
            ),
        ];
        let mut m = HashMap::new();
        for entry in initial_entries {
            m.insert(entry.0, entry.1.to_string());
        }

        RwLock::new(m)
    };
}
pub fn add_address_name(key: Address, value: String) {
    let mut map = ADDRESS_MAPPING.write().unwrap(); // Get write access to the map
    map.insert(key, value);
}

pub fn u256_to_address(input: U256) -> Address {
    Address::try_from(&input.to_be_bytes::<32>()[12..32]).unwrap()
}

pub fn address_to_human(address: &Address) -> String {
    let map = ADDRESS_MAPPING.read().unwrap(); // Get read access to the map
    if let Some(human_name) = map.get(address) {
        let tmp = address.to_string();

        format!(
            "{}...{} ({:^26})",
            &tmp[0..5],
            &tmp[tmp.len() - 5..tmp.len()],
            human_name
        )
    } else {
        format!("{}", address)
    }
}
