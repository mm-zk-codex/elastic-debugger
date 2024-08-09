use alloy::primitives::address;
use alloy::sol;
use colored::Colorize;
use sequencer::{detect_sequencer, SequencerType};

mod bridgehub;
mod sequencer;
mod statetransition;

sol! {
    #[sol(rpc)]
    contract SharedBridge {
        function assetHandlerAddress(bytes32 asset_id) public view returns (address) {}

    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("====================================");
    println!("=====   Elastic chain debugger =====");
    println!("====================================");

    let l1_sequencer = detect_sequencer("http://127.0.0.1:8545").await?;

    println!("{} L1 (ethereum) - {}", "[OK]".green(), l1_sequencer);

    let l2_sequencer = detect_sequencer("http://127.0.0.1:3050").await?;
    println!("{} L2 (gateway)  - {}", "[OK]".green(), l2_sequencer);

    // The client sequencer might not be running - but that's ok.
    let l3_sequencer = detect_sequencer("http://127.0.0.1:3060").await;
    match l3_sequencer {
        Ok(l3_sequencer) => println!("{} L3 (client)   - {}", "[OK]".green(), l3_sequencer),
        Err(err) => println!("{} L3 (client)   - {}", "[ERROR]".red(), err),
    };

    let l2_chain_id = l2_sequencer.chain_id;
    let l3_chain_id = 320;

    let info = match &l2_sequencer.sequencer_type {
        SequencerType::L1 => eyre::bail!("port 3050 doesn't have zksync sequencer"),
        SequencerType::L2(info) => info,
    };

    let bridgehub = bridgehub::Bridgehub::new(&l1_sequencer, info.bridgehub_address, true).await?;

    println!("===");
    println!("=== Bridehubs");
    println!("===");

    println!(
        "Found {} chains on L1 bridgehub: {:?}",
        bridgehub
            .known_chains
            .as_ref()
            .map(|x| x.len())
            .unwrap_or(0),
        bridgehub.known_chains
    );

    println!("Contracts on L1:");
    bridgehub.print_detailed_info().await?;

    let gateway_bridgehub_address = address!("0000000000000000000000000000000000010002");
    let mut gateway_bridgehub =
        bridgehub::Bridgehub::new(&l2_sequencer, gateway_bridgehub_address, true).await?;

    // HACK: currently we cannot autodetect chains that are in Gateway - as we don't publish any events.
    gateway_bridgehub.known_chains = Some([l3_chain_id].into());

    println!("L2 contracts on Gateway:");
    gateway_bridgehub.print_detailed_info().await?;

    println!("===");
    println!("=== State Transitions");
    println!("===");

    println!(
        "Chain 270 on L1: {}",
        bridgehub.get_state_transition(l2_chain_id).await?
    );
    println!(
        "Chain 320 on L1: {}",
        bridgehub.get_state_transition(l3_chain_id).await?
    );
    println!(
        "Chain 320 on Gateway: {}",
        gateway_bridgehub.get_state_transition(l3_chain_id).await?
    );
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
