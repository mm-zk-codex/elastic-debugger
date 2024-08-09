use std::fmt::Display;

use alloy::primitives::FixedBytes;
use alloy::primitives::{Address, U256};
use alloy::sol;

#[derive(Debug)]
pub struct STStorage {
    verifier: Address,
    total_batches_executed: U256,
    total_batches_verified: U256,
    total_batches_committed: U256,
    bootloader_hash: FixedBytes<32>,
    default_account_hash: FixedBytes<32>,
    protocol_version: (u32, u32, u32),
    system_upgrade_tx_hash: FixedBytes<32>,
    admin: Address,
    chain_id: U256,
    sync_layer: Address,
}

sol! {
    #[sol(rpc)]
    contract IHyperchain {
        function getVerifier() external view returns (address);
        function getAdmin() external view returns (address);
        function getTotalBatchesCommitted() external view returns (uint256);
        function getTotalBatchesVerified() external view returns (uint256);
        function getTotalBatchesExecuted() external view returns (uint256);
        function getSemverProtocolVersion() external view returns (uint32, uint32, uint32);

        function getL2BootloaderBytecodeHash() external view returns (bytes32);
        function getL2DefaultAccountBytecodeHash() external view returns (bytes32);
        function getL2SystemContractsUpgradeTxHash() external view returns (bytes32);
        function getChainId() external view returns (uint256);
        function getSyncLayer() external view returns (address);
    }
}

impl Display for STStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Chain id: {}", self.chain_id)?;
        // TODO: print proper protocol version.
        writeln!(
            f,
            "  Protocol version: {}.{}.{}",
            self.protocol_version.0, self.protocol_version.1, self.protocol_version.2
        )?;
        writeln!(
            f,
            "  Batches (C,V,E):  {} {} {}",
            self.total_batches_committed, self.total_batches_verified, self.total_batches_executed
        )?;

        writeln!(
            f,
            "  System upgrade:   {}",
            self.system_upgrade_tx_hash.to_string()
        )?;
        writeln!(
            f,
            "  AA hash:          {}",
            self.default_account_hash.to_string()
        )?;
        writeln!(f, "  Verifier:         {}", self.verifier)?;
        writeln!(f, "  Admin:            {}", self.admin)?;
        writeln!(
            f,
            "  Bootloader hash:  {}",
            self.bootloader_hash.to_string()
        )?;
        writeln!(f, "  Sync layer:  {}", self.sync_layer)
    }
}

pub async fn get_state_transition_storage(
    provider: &alloy::providers::RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
    >,
    hyperchain: Address,
) -> eyre::Result<STStorage> {
    let contract = IHyperchain::new(hyperchain, provider);

    let verifier = contract.getVerifier().call().await?._0;
    let total_batches_committed = contract.getTotalBatchesCommitted().call().await?._0;
    let total_batches_verified = contract.getTotalBatchesCommitted().call().await?._0;
    let total_batches_executed = contract.getTotalBatchesCommitted().call().await?._0;
    let protocol_version = contract.getSemverProtocolVersion().call().await?;

    let admin = contract.getAdmin().call().await?._0;

    let bootloader_hash = contract.getL2BootloaderBytecodeHash().call().await?._0;
    let default_account_hash = contract.getL2DefaultAccountBytecodeHash().call().await?._0;
    let system_upgrade_tx_hash = contract
        .getL2SystemContractsUpgradeTxHash()
        .call()
        .await?
        ._0;

    let chain_id = contract.getChainId().call().await?._0;
    let sync_layer = contract.getSyncLayer().call().await?._0;

    Ok(STStorage {
        verifier,
        total_batches_executed,
        total_batches_verified,
        total_batches_committed,
        bootloader_hash,
        default_account_hash,
        protocol_version: (
            protocol_version._0,
            protocol_version._1,
            protocol_version._2,
        ),
        system_upgrade_tx_hash,
        admin,
        chain_id,
        sync_layer,
    })
}
