use alloy::primitives::{address, Address, U256};
use alloy::sol;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use sequencer::{detect_sequencer, SequencerType};

mod addresses;
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

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    network: Option<Network>,

    #[arg(long)]
    bridgehub: Option<Address>,

    #[arg(long)]
    l1_url: Option<String>,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum Network {
    Local,
    Mainnet,
    Testnet,
    Stage,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Cli::parse();

    let (l1_rpc, l2_rpc, l3_rpc) = match args.network.clone().unwrap_or(Network::Local) {
        Network::Local => (
            "http://127.0.0.1:8545",
            "http://127.0.0.1:3150",
            "http://127.0.0.1:3050",
        ),
        Network::Mainnet => (
            //"https://rpc.flashbots.net",
            "https://eth.llamarpc.com",
            "https://rpc.era-gateway-mainnet.zksync.dev/",
            "https://mainnet.era.zksync.io",
        ),
        Network::Stage => (
            "https://1rpc.io/sepolia",
            "https://rpc.era-gateway-stage.zksync.dev/",
            "https://dev-api.era-stage-proofs.zksync.dev/",
        ),
        Network::Testnet => (
            "https://1rpc.io/sepolia",
            // TODO: for testnet, we'll have to point at the new testnet gateway once it's live
            "https://rpc.era-gateway-testnet.zksync.dev/",
            "https://sepolia.era.zksync.dev",
        ),
    };

    let l1_rpc = args.l1_url.as_deref().unwrap_or(l1_rpc);

    println!("====================================");
    println!("=====   Elastic chain debugger =====");
    println!("====================================");

    let l1_sequencer = detect_sequencer(l1_rpc).await?;

    println!("{} L1 (ethereum) - {}", "[OK]".green(), l1_sequencer);

    let l2_sequencer = detect_sequencer(l2_rpc).await;
    match &l2_sequencer {
        Ok(l2_sequencer) => println!("{} L2 (sequencer) - {}", "[OK]".green(), l2_sequencer),
        Err(err) => println!("{} L2 (sequencer) - {}", "[ERROR]".red(), err),
    };

    // The client sequencer might not be running - but that's ok.
    let l3_sequencer = detect_sequencer(l3_rpc).await;
    match &l3_sequencer {
        Ok(l3_sequencer) => println!("{} L3 (client)   - {}", "[OK]".green(), l3_sequencer),
        Err(err) => println!("{} L3 (client)   - {}", "[ERROR]".red(), err),
    };

    let bridgehub_address = match &l2_sequencer {
        Ok(l2_sequencer) => {
            if let SequencerType::L2(info) = &l2_sequencer.sequencer_type {
                info.bridgehub_address
            } else {
                eyre::bail!("port 3050 doesn't have zksync sequencer");
            }
        }
        Err(_) => {
            println!(
                "{} L2 (sequencer) missing - using L3 sequencer instead",
                "[ERROR]".red(),
            );
            if let Ok(l3_sequencer) = &l3_sequencer {
                if let SequencerType::L2(info) = &l3_sequencer.sequencer_type {
                    info.bridgehub_address
                } else {
                    eyre::bail!("port 3050 doesn't have zksync sequencer");
                }
            } else {
                eyre::bail!(
                    "L2 sequencer is not available and L3 sequencer is not a valid L2 sequencer"
                );
            }
        }
    };

    let bridgehub =
        bridgehub::Bridgehub::new(&l1_sequencer, args.bridgehub.unwrap_or(bridgehub_address))
            .await?;

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

    let gateway_bridgehub = match l2_sequencer {
        Ok(l2_sequencer) => {
            let gateway_bridgehub_address = address!("0000000000000000000000000000000000010002");
            let gateway_bridgehub =
                bridgehub::Bridgehub::new(&l2_sequencer, gateway_bridgehub_address).await?;

            println!("===");
            println!("=== {} ", format!("Bridgehub - Gateway").bold().green());
            println!("===");

            println!("{}", gateway_bridgehub);

            println!("\n=== Chains");
            gateway_bridgehub.print_detailed_info().await?;

            println!("===");
            println!("=== {} ", format!("ST / Hyperchains").bold().green());
            println!("===");
            Some(gateway_bridgehub)
        }
        Err(_) => None,
    };

    for chain in &bridgehub.known_chains {
        let st = bridgehub.get_state_transition(*chain).await;

        if let Ok(st) = st {
            print!("Chain {} on L1: {}", chain, st);
            if args.network.as_ref().unwrap_or(&Network::Local) == &Network::Local {
                // For L1 bridgehub - verify all the priority queue hashes.
                st.verify_priority_root_hash(&l1_sequencer).await?;
                println!("  Priority tree hash: {}", "VALID".green());
            } else {
                println!("  Skipping priority hash verification on non-local chains.");
            }
        } else {
            println!(
                "Failed to get info for Chain {} on L1: {}",
                chain,
                st.unwrap_err()
            );
        }

        println!("");
    }

    if let Some(gateway_bridgehub) = &gateway_bridgehub {
        for chain in &gateway_bridgehub.known_chains {
            println!(
                "Chain {} on Gateway: {}",
                chain,
                gateway_bridgehub.get_state_transition(*chain).await?
            );
        }
    }

    println!("===");
    println!("=== {} ", format!("Priority TXs").bold().green());
    println!("===");

    for chain in &bridgehub.known_chains {
        let st = bridgehub.get_state_transition(*chain).await;

        println!("Chain {}", chain);

        if let Ok(st) = st {
            let mut txs = st.get_priority_transactions(&l1_sequencer).await?;
            txs.sort_by_key(|x| x.index);
            for tx in txs {
                println!("{}", tx);
            }
            println!("");
        } else {
            println!(
                "Failed to get priority transactions for Chain {}: {}",
                chain,
                st.unwrap_err()
            );
        }
    }

    Ok(())
}
