use alloy::{
    primitives::{keccak256, Address, FixedBytes, B256},
    providers::Provider,
    rpc::types::{Filter, Log},
};
use names::{ADJECTIVES, NOUNS};

use crate::sequencer::Sequencer;

pub async fn get_all_events(
    sequencer: &Sequencer,
    address: Address,
    signature: B256,
) -> eyre::Result<Vec<Log>> {
    let provider = sequencer.get_provider();
    let mut current_block = provider.get_block_number().await?;
    let mut result = vec![];

    while current_block > 0 {
        let prev_limit = current_block.saturating_sub(10000);

        let filter = Filter::new()
            .from_block(prev_limit + 1)
            .to_block(current_block)
            .event_signature(signature)
            .address(address);

        let mut logs = sequencer.get_provider().get_logs(&filter).await?;
        result.append(&mut logs);
        current_block = prev_limit;
    }

    Ok(result)
}

pub fn get_human_name_for<T: AsRef<[u8]>>(entry: T) -> String {
    let hashed_address = keccak256(entry);
    let pos = usize::from_be_bytes(hashed_address[0..8].try_into().unwrap());
    format!(
        "{}_{}",
        ADJECTIVES[pos % ADJECTIVES.len()],
        NOUNS[pos % NOUNS.len()]
    )
}

pub fn address_from_fixedbytes(bytes: &FixedBytes<32>) -> eyre::Result<Address> {
    for i in 0..12 {
        if bytes.0[i] != 0 {
            eyre::bail!("cannot cast 32 bytes to address - non zero value in first 12 bytes");
        }
    }

    Ok(Address::from_slice(&bytes.0[12..32]))
}
