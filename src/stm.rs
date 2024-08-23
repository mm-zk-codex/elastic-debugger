use std::fmt::Display;

use alloy::{
    primitives::{Address, FixedBytes, U256},
    providers::Provider,
    rpc::types::Filter,
    sol,
    sol_types::SolEvent,
};
use colored::Colorize;

use crate::{bridgehub::IBridgehub, sequencer::Sequencer, utils::get_human_name_for};

sol! {
    #[sol(rpc)]
    contract IStateTransitionManager {
        event NewHyperchain(uint256 indexed _chainId, address indexed _hyperchainContract);
        function BRIDGE_HUB() external view returns (address);
        function admin() external view returns (address);
        function owner() external view returns (address);
    }
}

pub struct StateTransitionManager {
    pub address: Address,
    pub bridgehub: Address,
    pub admin: Address,
    pub owner: Address,
    pub asset_id: FixedBytes<32>,
    pub asset_name: String,
}

impl StateTransitionManager {
    pub async fn new(sequencer: &Sequencer, address: Address) -> Self {
        let provider = sequencer.get_provider();
        let contract = IStateTransitionManager::new(address, provider);

        let bridgehub = contract.BRIDGE_HUB().call().await.unwrap()._0;

        let admin = contract.admin().call().await.unwrap()._0;
        let owner = contract.owner().call().await.unwrap()._0;
        //let bridgehub = Address::ZERO;
        let provider = sequencer.get_provider();

        let bridgehub_contract = IBridgehub::new(bridgehub, provider);

        let asset_id = bridgehub_contract
            .stmAssetId(address)
            .call()
            .await
            .unwrap()
            ._0;
        let asset_name = get_human_name_for(asset_id);

        Self {
            address,
            bridgehub,
            admin,
            owner,
            asset_id,
            asset_name,
        }
    }
}

impl Display for StateTransitionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== STM -     {}", self.asset_name.bold().white())?;
        writeln!(f, "   Address:   {}", self.address)?;
        writeln!(f, "   Asset id:  {}", self.asset_id)?;
        writeln!(f, "   Bridgehub: {}", self.bridgehub)?;
        writeln!(f, "   Admin:     {}", self.admin)?;
        writeln!(f, "   Owner:     {}", self.owner)?;

        Ok(())
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
