use alloy::hex::FromHex;
use alloy::primitives::{address, Address, U256};
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

async fn get_bridgehub_contracts(
    provider: &alloy::providers::RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
    >,
    bridgehub: Address,
    chain_id: u64,
) -> eyre::Result<()> {
    sol! {
        #[sol(rpc)]
        contract Bridgehub {
            address public sharedBridge;
            mapping(uint256 chainId => address) public stateTransitionManager;
            mapping(uint256 chainId => address) public baseToken;
            function getHyperchain(uint256 _chainId) public view returns (address) {}

        }
    }

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
    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
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
    get_bridgehub_contracts(&l1_provider, bridgehub, chain_id).await?;

    let gateway_bridgehub_address = address!("0000000000000000000000000000000000010002");

    let l3_chain_id = 320;

    println!("L2 contracts on Gateway:");
    get_bridgehub_contracts(&l2_provider, gateway_bridgehub_address, l3_chain_id).await?;

    // TODO: add L3 too.

    Ok(())
}
