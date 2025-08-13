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
            .ctmAssetIdFromAddress(address)
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
