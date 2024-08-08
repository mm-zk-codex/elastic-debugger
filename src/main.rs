use alloy::hex::FromHex;
use alloy::primitives::{address, Address, U160, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::sol;
use colored::Colorize;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use std::net::TcpStream;
use std::time::Duration;

fn is_port_active(address: &str, port: u16) -> bool {
    let timeout = Duration::from_secs(1);
    let address = format!("{}:{}", address, port);
    match TcpStream::connect_timeout(&address.parse().unwrap(), timeout) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[derive(Deserialize, Debug)]
struct BridgehubResult {
    result: String,
}

const L1_PORT: u16 = 8545;
const GATEWAY_PORT: u16 = 3050;

async fn get_bridgehub_address(url: &str) -> eyre::Result<Address> {
    let client = Client::new();

    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "zks_getBridgehubContract",
        "params": []
    });

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let response_json: BridgehubResult = response.json().await?;
    Ok(Address::from_hex(response_json.result)?)
}

struct BridgehubAddresses {
    pub stm_address: Address,
    pub st_address: Address,
    pub shared_bridge_address: Address,
}

sol! {
    #[sol(rpc)]
    contract Bridgehub {
        address public sharedBridge;
        mapping(uint256 chainId => address) public stateTransitionManager;
        mapping(uint256 chainId => address) public baseToken;
        function getHyperchain(uint256 _chainId) public view returns (address) {}
        function stmAssetIdFromChainId(uint256 chain_id) public view returns (bytes32) {}

    }
}
sol! {
    #[sol(rpc)]
    contract SharedBridge {
        function assetHandlerAddress(bytes32 asset_id) public view returns (address) {}

    }
}

async fn get_bridgehub_contracts(
    provider: &alloy::providers::RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
    >,
    bridgehub: Address,
    chain_id: u64,
) -> eyre::Result<BridgehubAddresses> {
    sol! {
        #[sol(rpc)]
        contract StateTransitionManager {
            address public validatorTimelock;
        }
    }

    let contract = Bridgehub::new(bridgehub, provider);

    let stm_address = contract
        .stateTransitionManager(U256::from(chain_id))
        .call()
        .await?
        ._0;
    let base_token_address = contract.baseToken(U256::from(chain_id)).call().await?._0;
    let st_address = contract
        .getHyperchain(U256::from(chain_id))
        .call()
        .await?
        ._0;
    let shared_bridge_address = contract.sharedBridge().call().await?.sharedBridge;

    let stm_contract = StateTransitionManager::new(stm_address, provider);

    let validator_timelock_address = stm_contract
        .validatorTimelock()
        .call()
        .await?
        .validatorTimelock;

    println!("    Bridgehub:          {}", bridgehub);
    println!("    STM:                {}", stm_address);
    println!("    ST:                 {}", st_address);
    println!("    Base Token:         {}", base_token_address);
    println!("    Shared bridge:      {}", shared_bridge_address);
    println!("    Validator timelock: {}", validator_timelock_address);
    Ok(BridgehubAddresses {
        stm_address,
        st_address,
        shared_bridge_address,
    })
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
    if !is_port_active("127.0.0.1", L1_PORT) {
        println!(
            "{}",
            "[FAIL] - localhost:8545 is not active - cannot find L1".red()
        );
        return Ok(());
    }

    // Set up the HTTP transport which is consumed by the RPC client.
    let l1_rpc_url = format!("http://127.0.0.1:{L1_PORT}").parse()?;
    // Create a provider with the HTTP transport using the `reqwest` crate.
    let l1_provider = ProviderBuilder::new().on_http(l1_rpc_url);
    let latest_block = l1_provider.get_block_number().await?;

    println!(
        "{} - L1 found on 8545. Latest block: {}",
        "[OK]".green(),
        latest_block
    );

    if !is_port_active("127.0.0.1", GATEWAY_PORT) {
        println!(
            "{}",
            "[FAIL] - localhost:3050 is not active - cannot find Gateway".red()
        );
        return Ok(());
    }

    let l2_rpc_url = format!("http://127.0.0.1:{GATEWAY_PORT}").parse()?;
    // Create a provider with the HTTP transport using the `reqwest` crate.
    let l2_provider: alloy::providers::RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
    > = ProviderBuilder::new().on_http(l2_rpc_url);

    let latest_block = l2_provider.get_block_number().await?;
    let chain_id = l2_provider.get_chain_id().await?;
    println!(
        "{} - Gateway found on 3050. Latest block: {}, Chain id: {}",
        "[OK]".green(),
        latest_block,
        chain_id
    );

    let url = format!("http://127.0.0.1:{GATEWAY_PORT}");
    let bridgehub = get_bridgehub_address(&url).await?;

    println!("Gateway contracts on L1:");
    let l1_bh_addresses = get_bridgehub_contracts(&l1_provider, bridgehub, chain_id).await?;

    let gateway_bridgehub_address = address!("0000000000000000000000000000000000010002");

    let l3_chain_id = 320;

    println!("L2 contracts on Gateway:");
    let l2_bh_addresses =
        get_bridgehub_contracts(&l2_provider, gateway_bridgehub_address, l3_chain_id).await?;

    let h1_storage = get_hyperchain_storage(&l1_provider, l1_bh_addresses.st_address).await?;
    let h2_storage = get_hyperchain_storage(&l2_provider, l2_bh_addresses.st_address).await?;

    println!("h1 storage: {:?}", h1_storage);
    println!("h2 storage: {:?}", h2_storage);

    let contract = Bridgehub::new(bridgehub, &l1_provider);
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

    println!("L3 asset handler: {:?}", l3_asset_handler);

    // TODO: add L3 too.

    Ok(())
}
