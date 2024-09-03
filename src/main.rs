use alloy::primitives::{address, U256};
use alloy::sol;
use colored::Colorize;
use sequencer::{detect_sequencer, SequencerType};

mod bridgehub;
mod l1_asset_router;
mod l2_asset_router;
mod priority_transactions;
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

fn format_wei_amount(wei: &U256) -> String {
    let wei_string = wei.to_string();
    let len = wei_string.len();

    if len > 18 {
        // Insert a decimal 18 places from the end
        format!("{}.{}", &wei_string[..len - 18], &wei_string[len - 18..])
    } else {
        // If the string is shorter than 18 characters, pad with zeros
        format!(
            "0.{}",
            wei_string
                .chars()
                .rev()
                .chain("000000000000000000".chars())
                .take(18)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        )
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
    println!("=== {} ", format!("Bridgehub - L1").bold().green());
    println!("===");

    println!("{}", bridgehub);

    println!("=== Bridgehub chains");
    bridgehub.print_detailed_info().await?;

    println!("=== Balances ");

    let balances = bridgehub
        .get_all_chains_balances(&l1_sequencer)
        .await
        .unwrap();
    for (chain, balance) in balances.iter() {
        println!("   Chain : {}", format!("{}", chain).bold());
        for (token, amount) in balance.iter() {
            println!(
                "      {:<20} : {:>28}",
                token.bold(),
                format_wei_amount(amount)
            );
        }
    }

    let gateway_bridgehub_address = address!("0000000000000000000000000000000000010002");
    let gateway_bridgehub =
        bridgehub::Bridgehub::new(&l2_sequencer, gateway_bridgehub_address, true).await?;

    println!("===");
    println!("=== {} ", format!("Bridgehub - Gateway").bold().green());
    println!("===");

    println!("{}", gateway_bridgehub);

    println!("\n=== Chains");
    gateway_bridgehub.print_detailed_info().await?;

    println!("===");
    println!("=== {} ", format!("ST / Hyperchains").bold().green());
    println!("===");

    if let Some(chains) = &bridgehub.known_chains {
        for chain in chains {
            let st = bridgehub.get_state_transition(*chain).await?;

            print!("Chain {} on L1: {}", chain, st);
            // For L1 bridgehub - verify all the priority queue hashes.
            st.verify_priority_root_hash(&l1_sequencer).await?;
            println!("  Priority tree hash: {}", "VALID".green());
            println!("");
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
