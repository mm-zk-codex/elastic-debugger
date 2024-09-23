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
    contract IChainTypeManager {
        event NewHyperchain(uint256 indexed _chainId, address indexed _hyperchainContract);
        event MigrationFinalized(uint256 indexed chainId, bytes32 indexed assetId, address indexed zkChain);
        function BRIDGE_HUB() external view returns (address);
        function admin() external view returns (address);
        function owner() external view returns (address);
    }
}

pub struct ChainTypeManager {
    pub address: Address,
    pub bridgehub: Address,
    pub admin: Address,
    pub owner: Address,
    pub asset_id: FixedBytes<32>,
    pub asset_name: String,
}

impl ChainTypeManager {
    pub async fn new(sequencer: &Sequencer, address: Address) -> Self {
        let provider = sequencer.get_provider();
        let contract = IChainTypeManager::new(address, provider);

        let bridgehub = contract.BRIDGE_HUB().call().await.unwrap()._0;

        let admin = contract.admin().call().await.unwrap()._0;
        let owner = contract.owner().call().await.unwrap()._0;
        let provider = sequencer.get_provider();

        let bridgehub_contract = IBridgehub::new(bridgehub, provider);

        let asset_id = bridgehub_contract
            .ctmAssetId(address)
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

    pub fn detailed_fmt(&self, f: &mut std::fmt::Formatter<'_>, pad: usize) -> std::fmt::Result {
        let pad = " ".repeat(pad);
        writeln!(f, "{}=== CTM -     {}", pad, self.asset_name.bold().white())?;
        writeln!(f, "{}   Address:   {}", pad, self.address)?;
        writeln!(f, "{}   Asset id:  {}", pad, self.asset_id)?;
        writeln!(f, "{}   Bridgehub: {}", pad, self.bridgehub)?;
        writeln!(f, "{}   Admin:     {}", pad, self.admin)?;
        writeln!(f, "{}   Owner:     {}", pad, self.owner)?;

        Ok(())
    }
}

impl Display for ChainTypeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.detailed_fmt(f, 0)
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
            .event_signature(IChainTypeManager::MigrationFinalized::SIGNATURE_HASH);

        let logs = sequencer.get_provider().get_logs(&filter).await?;
        for log in logs {
            let chain_id: U256 = log.topics()[1].into();
            let chain_id = U256::to::<u64>(&chain_id);

            let st_address: alloy::primitives::FixedBytes<32> = log.topics()[3];
            let st_address: [u8; 20] = st_address.0[12..].try_into().unwrap();
            let st_address = Address::from(st_address);

            result.push((chain_id, st_address));
        }
        current_block = prev_limit;
    }

    Ok(result)
}
