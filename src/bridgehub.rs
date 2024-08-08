use std::collections::HashSet;
use std::fmt::Display;

use alloy::primitives::{Address, IntoLogData, U256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;

use crate::sequencer::Sequencer;

sol! {
    #[sol(rpc)]
    contract IBridgehub {
        address public sharedBridge;
        mapping(uint256 chainId => address) public stateTransitionManager;
        mapping(uint256 chainId => address) public baseToken;
        function getHyperchain(uint256 _chainId) public view returns (address) {}
        function stmAssetIdFromChainId(uint256 chain_id) public view returns (bytes32) {}

        event NewChain(uint256 indexed chainId, address stateTransitionManager, address indexed chainGovernance);
        event AssetRegistered(
            bytes32 indexed assetInfo,
            address indexed _assetAddress,
            bytes32 indexed additionalData,
            address sender
        );


    }
}

pub struct Bridgehub {
    pub address: Address,
    pub shared_bridge: Address,
    pub known_chains: Option<HashSet<u64>>,
}
pub struct BridgehubAddresses {
    pub stm_address: Address,
    pub st_address: Address,
    pub shared_bridge_address: Address,
}

impl Display for Bridgehub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bridgehub at {}. Shared bridge: {}",
            self.address, self.shared_bridge
        )
    }
}

impl Bridgehub {
    pub async fn new(sequencer: &Sequencer, address: Address) -> eyre::Result<Bridgehub> {
        let provider = sequencer.get_provider();

        let data = provider.get_code_at(address).await?;
        if data.len() == 0 {
            // empty contract - something's wrong.
            eyre::bail!(
                "Trying to read bridgehub data from address {} at {}, but code is empty. Is it a rigth address on right chain?",
                address,
                provider.client().transport().url()
            );
        }

        let contract = IBridgehub::new(address, provider);
        let shared_bridge = contract.sharedBridge().call().await?.sharedBridge;

        Ok(Bridgehub {
            address,
            shared_bridge,
            known_chains: None,
        })
    }

    pub async fn update_chains(&mut self, sequencer: &Sequencer) -> eyre::Result<()> {
        let chains = self.detect_chains(sequencer).await?;
        self.known_chains = Some(chains);
        Ok(())
    }

    pub async fn detect_chains(&self, sequencer: &Sequencer) -> eyre::Result<HashSet<u64>> {
        let provider = sequencer.get_provider();
        let mut current_block = provider.get_block_number().await?;
        let mut known_chains = HashSet::new();

        while current_block > 0 {
            let prev_limit = current_block.saturating_sub(10000);

            let filter = Filter::new()
                .from_block(prev_limit + 1)
                .to_block(current_block)
                .address(self.address);

            let logs = sequencer.get_provider().get_logs(&filter).await?;
            for log in logs {
                match log.topic0() {
                    Some(&IBridgehub::NewChain::SIGNATURE_HASH) => {
                        let chain_id: U256 = log.topics()[1].into();
                        known_chains.insert(chain_id.to::<u64>());
                    }

                    Some(&IBridgehub::AssetRegistered::SIGNATURE_HASH) => {
                        // TODO: do something with assets.
                        // println!("New asset. {:?}", log);
                    }
                    _ => (),
                }
            }
            current_block = prev_limit;
        }

        Ok(known_chains)
    }

    pub async fn get_bridgehub_contracts(
        &self,
        provider: &alloy::providers::RootProvider<
            alloy::transports::http::Http<alloy::transports::http::Client>,
        >,
        chain_id: u64,
    ) -> eyre::Result<BridgehubAddresses> {
        sol! {
            #[sol(rpc)]
            contract StateTransitionManager {
                address public validatorTimelock;
            }
        }

        let contract = IBridgehub::new(self.address, provider);

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

        println!("    Bridgehub:          {}", self.address);
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
}
