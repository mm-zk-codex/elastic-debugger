use alloy::primitives::{address, Address, U160, U256};
use alloy::providers::Provider;
use alloy::sol;
use colored::Colorize;
use sequencer::{detect_sequencer, SequencerType};

mod bridgehub;
mod sequencer;

sol! {
    #[sol(rpc)]
    contract SharedBridge {
        function assetHandlerAddress(bytes32 asset_id) public view returns (address) {}

    }
}

#[derive(Debug)]
struct HyperchainStorage {
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

async fn get_hyperchain_storage(
    provider: &alloy::providers::RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
    >,
    hyperchain: Address,
) -> eyre::Result<HyperchainStorage> {
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

    Ok(HyperchainStorage {
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

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("=====   Elastic chain debugger =====");

    let l1_sequencer = detect_sequencer("http://127.0.0.1:8545").await?;

    println!("{} L1 (ethereum) - {}", "[OK]".green(), l1_sequencer);

    let l1_provider = l1_sequencer.get_provider();

    let l2_sequencer = detect_sequencer("http://127.0.0.1:3050").await?;
    println!("{} L2 (gateway)  - {}", "[OK]".green(), l2_sequencer);

    let l2_provider = l2_sequencer.get_provider();
    let chain_id = l2_sequencer.chain_id;

    let info = match &l2_sequencer.sequencer_type {
        SequencerType::L1 => eyre::bail!("port 3050 doesn't have zksync sequencer"),
        SequencerType::L2(info) => info,
    };

    let bridgehub = bridgehub::Bridgehub::new(&l1_sequencer, info.bridgehub_address, true).await?;
    println!("Bridgehub: {}", bridgehub);

    println!("Found chains: {:?}", bridgehub.known_chains);

    println!("Gateway contracts on L1:");
    bridgehub.print_detailed_info(&l1_provider).await?;
    let l1_bh_addresses = bridgehub
        .get_bridgehub_contracts(&l1_provider, chain_id)
        .await?;

    let gateway_bridgehub_address = address!("0000000000000000000000000000000000010002");
    let mut gateway_bridgehub =
        bridgehub::Bridgehub::new(&l2_sequencer, gateway_bridgehub_address, true).await?;

    // HACK: currently we cannot autodetect chains that are in Gateway - as we don't publish any events.
    gateway_bridgehub.known_chains = Some([320].into());

    let l3_chain_id = 320;

    println!("L2 contracts on Gateway:");
    gateway_bridgehub.print_detailed_info(&l2_provider).await?;

    let l2_bh_addresses = gateway_bridgehub
        .get_bridgehub_contracts(&l2_provider, l3_chain_id)
        .await?;

    let h1_storage = get_hyperchain_storage(&l1_provider, l1_bh_addresses.st_address).await?;
    let h2_storage = get_hyperchain_storage(&l2_provider, l2_bh_addresses.st_address).await?;

    println!("h1 storage: {:?}", h1_storage);
    println!("h2 storage: {:?}", h2_storage);
    /*
    let contract = IBridgehub::new(bridgehub, &l1_provider);
    let stm_asset_l3 = contract
        .stmAssetIdFromChainId(U256::from(l3_chain_id))
        .call()
        .await?
        ._0;
    println!("Asset id {:?}", stm_asset_l3);

    let shared_bridge_contract =
        SharedBridge::new(l1_bh_addresses.shared_bridge_address, &l1_provider);

    let l3_asset_handler = shared_bridge_contract
        .assetHandlerAddress(stm_asset_l3)
        .call()
        .await?
        ._0;

    println!("L3 asset handler: {:?}", l3_asset_handler);*/

    /*
    let filter = Filter::new()
        .from_block(1)
        .to_block(5000)
        .address(address!("9cAC3E80223AF3aF00d591e53336CBe05953c0a0"))
        .event("NewChain(uint256,address,address)");
    let logs = l1_provider.get_logs(&filter).await?;
    for log in logs {
        println!("{:?}", log);
    }*/

    // TODO: add L3 too.

    Ok(())
}
