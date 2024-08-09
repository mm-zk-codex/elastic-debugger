use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    rpc::types::Filter,
    sol,
    sol_types::SolEvent,
};

use crate::sequencer::Sequencer;

sol! {
    #[sol(rpc)]
    contract IStateTransitionManager {
        event NewHyperchain(uint256 indexed _chainId, address indexed _hyperchainContract);
    }
}

// Scans all the events looking for NewHyperchain events.
pub async fn detect_hyperchains(sequencer: &Sequencer) -> eyre::Result<Vec<(u64, Address)>> {
    let provider = sequencer.get_provider();
    let mut current_block = provider.get_block_number().await?;
    let mut result = vec![];

    while current_block > 0 {
        let prev_limit = current_block.saturating_sub(10000);

        let filter = Filter::new()
            .from_block(prev_limit + 1)
            .to_block(current_block)
            .event_signature(IStateTransitionManager::NewHyperchain::SIGNATURE_HASH);

        let logs = sequencer.get_provider().get_logs(&filter).await?;
        for log in logs {
            let chain_id: U256 = log.topics()[1].into();
            let chain_id = U256::to::<u64>(&chain_id);

            let st_address: alloy::primitives::FixedBytes<32> = log.topics()[2];
            let st_address: [u8; 20] = st_address.0[12..].try_into().unwrap();
            let st_address = Address::from(st_address);
            result.push((chain_id, st_address));
        }
        current_block = prev_limit;
    }

    Ok(result)
}
