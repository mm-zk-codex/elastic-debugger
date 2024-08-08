use std::fmt::Display;

use alloy::{
    primitives::{Address, U160, U256},
    providers::Provider,
};

#[derive(Debug)]
pub struct STStorage {
    verifier: Address,
    total_batches_executed: U256,
    total_batches_verified: U256,
    total_batches_committed: U256,
    bootloader_hash: U256,
    default_account_hash: U256,
    protocol_version: U256,
    system_upgrade_tx_hash: U256,
    admin: Address,
    chain_id: U256,
}

fn protocol_version_to_string(protocol_version: U256) -> String {
    if protocol_version <= U256::from(24) {
        return protocol_version.to_string();
    }
    let (minor, patch) = protocol_version.div_rem(U256::from(1u64 << 32));

    return format!("{}.{}", minor, patch);
}

impl Display for STStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Chain id: {}", self.chain_id)?;
        // TODO: print proper protocol version.
        writeln!(
            f,
            "  Protocol version: {}",
            protocol_version_to_string(self.protocol_version)
        )?;
        writeln!(
            f,
            "  Batches (C,V,E): {} {} {}",
            self.total_batches_committed, self.total_batches_verified, self.total_batches_executed
        )?;

        writeln!(
            f,
            "  System upgrade: {}",
            self.system_upgrade_tx_hash.to_string()
        )?;
        writeln!(f, "  AA hash: {}", self.default_account_hash.to_string())?;
        writeln!(f, "  Verifier: {}", self.verifier)?;
        writeln!(f, "  Admin: {}", self.admin)?;
        writeln!(f, "  Bootloader hash: {}", self.bootloader_hash.to_string())
    }
}

pub async fn get_state_transition_storage(
    provider: &alloy::providers::RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
    >,
    hyperchain: Address,
) -> eyre::Result<STStorage> {
    const VERIFIER_SLOT: u32 = 10;
    const TOTAL_BATCHES_EXEC_SLOT: u32 = 11;
    const TOTAL_BATCHES_VERIFIED_SLOT: u32 = 12;
    const TOTAL_BATCHES_COMMITTED_SLOT: u32 = 13;
    const BOOTLOADER_SLOT: u32 = 23;
    const DEFAULT_AA_SLOT: u32 = 24;
    const PROTOCOL_VERSION_SLOT: u32 = 33;
    const SYSTEM_UPGRADE_TX_HASH_SLOT: u32 = 34;
    const ADMIN_SLOT: u32 = 36;
    // TODO: fee params
    const CHAIN_ID_SLOT: u32 = 40;

    let get_storage = |slot| provider.get_storage_at(hyperchain, U256::from(slot));

    Ok(STStorage {
        verifier: Address::from(U160::from(get_storage(VERIFIER_SLOT).await.unwrap())),
        total_batches_executed: get_storage(TOTAL_BATCHES_EXEC_SLOT).await.unwrap(),
        total_batches_verified: get_storage(TOTAL_BATCHES_VERIFIED_SLOT).await.unwrap(),
        total_batches_committed: get_storage(TOTAL_BATCHES_COMMITTED_SLOT).await.unwrap(),
        bootloader_hash: get_storage(BOOTLOADER_SLOT).await.unwrap(),
        default_account_hash: get_storage(DEFAULT_AA_SLOT).await.unwrap(),
        protocol_version: get_storage(PROTOCOL_VERSION_SLOT).await.unwrap(),
        system_upgrade_tx_hash: get_storage(SYSTEM_UPGRADE_TX_HASH_SLOT).await.unwrap(),
        admin: Address::from(U160::from(get_storage(ADMIN_SLOT).await.unwrap())),
        chain_id: get_storage(CHAIN_ID_SLOT).await.unwrap(),
    })
}
