use std::collections::HashMap;
use std::fmt::{Debug, Display};

use crate::addresses::{address_to_human, u256_to_address};
use crate::{sequencer::Sequencer, utils::get_all_events};
use alloy::primitives::{keccak256, Address, B256, U256};
use alloy::rpc::types::Log;
use alloy::sol;
use alloy::sol_types::SolEvent;
use colored::Colorize;
use lazy_static::lazy_static;

sol! {
    struct L2CanonicalTransaction {
        uint256 txType;
        uint256 from;
        uint256 to;
        uint256 gasLimit;
        uint256 gasPerPubdataByteLimit;
        uint256 maxFeePerGas;
        uint256 maxPriorityFeePerGas;
        uint256 paymaster;
        uint256 nonce;
        uint256 value;
        // In the future, we might want to add some
        // new fields to the struct. The `txData` struct
        // is to be passed to account and any changes to its structure
        // would mean a breaking change to these accounts. To prevent this,
        // we should keep some fields as "reserved"
        // It is also recommended that their length is fixed, since
        // it would allow easier proof integration (in case we will need
        // some special circuit for preprocessing transactions)
        uint256[4] reserved;
        bytes data;
        bytes signature;
        uint256[] factoryDeps;
        bytes paymasterInput;
        // Reserved dynamic type for the future use-case. Using it should be avoided,
        // But it is still here, just in case we want to enable some additional functionality
        bytes reservedDynamic;
    }

    #[sol(rpc)]
    contract IMailbox {
        event NewPriorityRequest(
        uint256 txId,
        bytes32 txHash,
        uint64 expirationTimestamp,
        L2CanonicalTransaction transaction,
        bytes[] factoryDeps
    );
}
}

lazy_static! {
    static ref KNOWN_SIGNATURES: HashMap<String, String> = {
        let json_value = serde_json::from_slice(include_bytes!("data/abi_map.json")).unwrap();
        let pairs: HashMap<String, String> = serde_json::from_value(json_value).unwrap();

        pairs
    };
}

pub struct PriorityTransaction {
    pub index: u64,
    tx_id: B256,
    expiration_timestamp: u64,
    l2_tx: L2CanonicalTransaction,
}

impl Debug for PriorityTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PriorityTransaction")
            .field("index", &self.index)
            .field("tx_id", &self.tx_id)
            .field("expiration_timestamp", &self.expiration_timestamp)
            .finish()
    }
}

impl Display for PriorityTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.detailed_fmt(f, 0)
    }
}

fn format_integer_with_underscores(input: &str) -> String {
    let reversed_input: String = input.chars().rev().collect();

    // Insert underscores every three characters
    let mut formatted = String::new();
    for (index, char) in reversed_input.chars().enumerate() {
        if index % 3 == 0 && index != 0 {
            formatted.push('_');
        }
        formatted.push(char);
    }

    // Reverse the formatted string to correct the order
    formatted.chars().rev().collect()
}

pub fn wei_as_string(value: U256) -> String {
    format_integer_with_underscores(&value.to_string())
}

impl PriorityTransaction {
    pub fn detailed_fmt(&self, f: &mut std::fmt::Formatter<'_>, pad: usize) -> std::fmt::Result {
        let pad = " ".repeat(pad);
        writeln!(f, "{}Tx: {} - {}", pad, self.index, self.tx_id)?;

        writeln!(
            f,
            "{}    {} -> {}",
            pad,
            address_to_human(&u256_to_address(self.l2_tx.from)),
            address_to_human(&u256_to_address(self.l2_tx.to))
        )?;

        if self.l2_tx.data.len() > 4 {
            let selector = hex::encode(&self.l2_tx.data[0..4]);
            let entry = KNOWN_SIGNATURES.get(&selector).unwrap_or(&selector);

            writeln!(f, "{}    Method           - {}", pad, entry.bold())?;
        }

        if self.l2_tx.reserved[0] > U256::ZERO {
            writeln!(
                f,
                "{}    Value (reserved) - {}",
                pad,
                wei_as_string(self.l2_tx.reserved[0])
            )?;
        }
        Ok(())
    }
}

impl From<Log> for PriorityTransaction {
    fn from(value: Log) -> Self {
        let request =
            IMailbox::NewPriorityRequest::abi_decode_data(&value.data().data, true).unwrap();

        let index: u64 = request.0.try_into().unwrap();
        let tx_id = request.1;
        let expiration_timestamp = request.2;

        Self {
            index,
            tx_id,
            expiration_timestamp,
            l2_tx: request.3,
        }
    }
}

pub fn compute_merkle_tree(txs: &Vec<PriorityTransaction>) -> B256 {
    let size = txs.len().next_power_of_two();
    let mut leaves = vec![keccak256(""); size];
    for tx in txs {
        leaves[tx.index as usize] = tx.tx_id;
    }
    while leaves.len() > 1 {
        let mut parents = vec![];

        for i in 0..(leaves.len() / 2) {
            let payload = [leaves[2 * i].as_slice(), leaves[2 * i + 1].as_slice()].concat();

            parents.push(keccak256(payload));
        }
        leaves = parents;
    }

    *leaves.get(0).unwrap()
}

pub async fn fetch_all_priority_transactions(
    sequencer: &Sequencer,
    address: Address,
) -> eyre::Result<Vec<PriorityTransaction>> {
    match sequencer.sequencer_type {
        crate::sequencer::SequencerType::L1 => {
            let events = get_all_events(
                sequencer,
                address,
                IMailbox::NewPriorityRequest::SIGNATURE_HASH,
                5000, // 5k block limit
            )
            .await
            .unwrap();
            let txs: Vec<PriorityTransaction> = events
                .into_iter()
                .map(|x| PriorityTransaction::from(x))
                .collect();

            Ok(txs)
        }
        crate::sequencer::SequencerType::L2(_) => {
            eyre::bail!("Priority transactions are only available on L1");
        }
    }
}
