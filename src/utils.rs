use alloy::{
    primitives::{Address, B256},
    providers::Provider,
    rpc::types::{Filter, Log},
};

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
