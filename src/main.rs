use alloy::primitives::address;
use alloy::sol;
use colored::Colorize;
use sequencer::{detect_sequencer, SequencerType};

mod bridgehub;
mod sequencer;
mod statetransition;
mod stm;
mod utils;

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

    let info = match &l2_sequencer.sequencer_type {
        SequencerType::L1 => eyre::bail!("port 3050 doesn't have zksync sequencer"),
        SequencerType::L2(info) => info,
    };

    let bridgehub = bridgehub::Bridgehub::new(&l1_sequencer, info.bridgehub_address, true).await?;

    println!("===");
    println!("=== Bridgehubs");
    println!("===");

    println!("{}", bridgehub);

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
    let gateway_bridgehub =
        bridgehub::Bridgehub::new(&l2_sequencer, gateway_bridgehub_address, true).await?;

    println!("{}", gateway_bridgehub);

    println!(
        "Found {} chains on Gateway bridgehub: {:?}",
        gateway_bridgehub
            .known_chains
            .as_ref()
            .map(|x| x.len())
            .unwrap_or(0),
        gateway_bridgehub.known_chains
    );

    println!("L2 contracts on Gateway:");
    gateway_bridgehub.print_detailed_info().await?;

    println!("===");
    println!("=== State Transitions");
    println!("===");

    if let Some(chains) = &bridgehub.known_chains {
        for chain in chains {
            println!(
                "Chain {} on L1: {}",
                chain,
                bridgehub.get_state_transition(*chain).await?
            );
        }
    }

    if let Some(chains) = &gateway_bridgehub.known_chains {
        for chain in chains {
            println!(
                "Chain {} on Gateway: {}",
                chain,
                gateway_bridgehub.get_state_transition(*chain).await?
            );
        }
    }

    Ok(())
}
