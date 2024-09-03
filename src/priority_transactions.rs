use crate::{sequencer::Sequencer, utils::get_all_events};
use alloy::primitives::{keccak256, Address, B256};
use alloy::rpc::types::Log;
use alloy::sol;
use alloy::sol_types::SolEvent;

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

#[derive(Debug)]
pub struct PriorityTransaction {
    index: u64,
    tx_id: B256,
    expiration_timestamp: u64,
}

impl From<Log> for PriorityTransaction {
    fn from(value: Log) -> Self {
        let data = &value.data().data;
        let index = u64::from_be_bytes(data[24..32].try_into().unwrap());
        let tx_id = B256::from_slice(data[32..64].try_into().unwrap());
        let expiration_timestamp = u64::from_be_bytes(data[88..96].try_into().unwrap());

        Self {
            index,
            tx_id,
            expiration_timestamp,
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
