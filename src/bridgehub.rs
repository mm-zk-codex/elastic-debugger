use std::collections::{HashMap, HashSet};
use std::fmt::Display;

use crate::l1_asset_router::{AssetHandler, L1AssetRouter};
use crate::l2_asset_router::L2AssetRouter;
use crate::sequencer::Sequencer;
use crate::statetransition::StateTransition;
use crate::stm::ChainTypeManager;
use crate::utils::get_human_name_for;
use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{Provider, RootProvider};
use alloy::sol;
use alloy::transports::http::{Client, Http};
use colored::Colorize;

use futures::future::join_all;

sol! {
    #[sol(rpc)]
    contract IBridgehub {
        address public sharedBridge;
        mapping(uint256 chainId => address) public chainTypeManager;
        mapping(uint256 chainId => address) public baseToken;
        function getHyperchain(uint256 _chainId) public view returns (address) {}
        function ctmAssetIdFromChainId(uint256 chain_id) public view returns (bytes32) {}

        function ctmAssetIdFromAddress(address) public view returns (bytes32) {}

        event NewChain(uint256 indexed chainId, address chainTypeManager, address indexed chainGovernance);
        event AssetRegistered(
            bytes32 indexed assetInfo,
            address indexed _assetAddress,
            bytes32 indexed additionalData,
            address sender
        );

        event ChainTypeManagerAdded(address indexed chainTypeManager);

        event ChainTypeManagerRemoved(address indexed chainTypeManager);

        function getAllZKChainChainIDs() external view returns (uint256[] memory);

        address public l1CtmDeployer;
    }
}

// Information about a single chain that is connected to a bridgehub.
// The chain_id is supposed to be a globally unique identifier.
// Note, that this object might exist in 'passive' mode - if the chain has migrated to a different sync layer.
pub struct BridgehubChainDetails {
    pub stm_address: Address,
    pub st_address: Address,
    pub base_token_address: Address,
    pub validator_timelock_address: Address,
    pub stm_asset_id: FixedBytes<32>,
}

impl Display for BridgehubChainDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "    CTM:      {}",
            get_human_name_for(self.stm_asset_id).bold()
        )?;
        writeln!(f, "    CTM:                {}", self.stm_address)?;
        writeln!(f, "    ST:                 {}", self.st_address)?;
        writeln!(f, "    Base Token:         {}", self.base_token_address)?;
        writeln!(
            f,
            "    Validator timelock: {}",
            self.validator_timelock_address
        )?;
        Ok(())
    }
}

pub enum AssetRouter {
    L1(L1AssetRouter),
    L2(L2AssetRouter),
}

impl Display for AssetRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.detailed_fmt(f, 0)
    }
}

impl AssetRouter {
    fn detailed_fmt(&self, f: &mut std::fmt::Formatter<'_>, pad_size: usize) -> std::fmt::Result {
        let pad = " ".repeat(pad_size);
        match self {
            AssetRouter::L1(router) => {
                writeln!(f, "{}L1 asset router", pad)?;
                router.detailed_fmt(f, pad_size + 3)?;
            }
            AssetRouter::L2(router) => {
                writeln!(f, "{}L2 asset router", pad)?;
                router.detailed_fmt(f, pad_size + 3)?;
            }
        }

        Ok(())
    }
}

/// Bridgehub is the main coordination contract on each chain.
/// the 'main main' bridgehub is located on L1.
pub struct Bridgehub {
    pub address: Address,
    pub shared_bridge: Address,
    pub known_chains: HashSet<u64>,
    pub ctms: Option<Vec<ChainTypeManager>>,
    provider: RootProvider<Http<Client>>,
    pub ctm_deployer: Address,

    pub asset_router: AssetRouter,
}

impl Display for Bridgehub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "   Bridgehub at          {}", self.address,)?;
        writeln!(f, "   Shared bridge:        {}", self.shared_bridge)?;
        writeln!(f, "   CTM deployer (on L1): {}", self.ctm_deployer)?;
        if let Some(ctms) = &self.ctms {
            writeln!(f, "   CTMS: {}", ctms.len())?;

            for stm in ctms {
                stm.detailed_fmt(f, 3)?;
            }
        }

        writeln!(f, "   === Asset router")?;
        self.asset_router.detailed_fmt(f, 3)?;

        Ok(())
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

        let known_chains = contract.getAllZKChainChainIDs().call().await?._0;

        let known_chains: HashSet<u64> =
            known_chains.iter().map(|x| x.try_into().unwrap()).collect();

        let ctm_deployer = contract.l1CtmDeployer().call().await?.l1CtmDeployer;

        let mut ctm_addresses = HashSet::new();

        for chain_id in known_chains.iter() {
            let aa = contract
                .chainTypeManager(U256::from(*chain_id))
                .call()
                .await
                .map(|x| x._0)
                .unwrap();
            ctm_addresses.insert(aa);
        }

        let ctms = {
            let stms = ctm_addresses
                .into_iter()
                .map(|address| ChainTypeManager::new(sequencer, address));
            let stms = join_all(stms).await;
            Some(stms)
        };

        let asset_router = match sequencer.sequencer_type {
            crate::sequencer::SequencerType::L1 => {
                AssetRouter::L1(L1AssetRouter::new(sequencer, shared_bridge).await?)
            }
            crate::sequencer::SequencerType::L2(_) => {
                AssetRouter::L2(L2AssetRouter::new(sequencer, shared_bridge).await)
            }
        };

        Ok(Bridgehub {
            address,
            shared_bridge,
            known_chains,
            provider: sequencer.get_provider(),
            ctms,
            ctm_deployer,
            asset_router,
        })
    }

    pub async fn print_detailed_info(&self) -> eyre::Result<()> {
        println!("  Bridgehub:          {}", self.address);

        for chain_id in &self.known_chains {
            println!("{}", format!("  Chain: {:?}", chain_id).bold());
            let details = self.get_chain_details(*chain_id).await?;
            println!("{}", details);
        }

        Ok(())
    }

    pub async fn get_chain_details(&self, chain_id: u64) -> eyre::Result<BridgehubChainDetails> {
        sol! {
            #[sol(rpc)]
            contract IChainTypeManager {
                address public validatorTimelock;
            }
        }

        let contract = IBridgehub::new(self.address, &self.provider);

        let stm_address = contract
            .chainTypeManager(U256::from(chain_id))
            .call()
            .await?
            ._0;

        let base_token_address = match contract.baseToken(U256::from(chain_id)).call().await {
            Ok(base_token) => base_token._0,
            // FIXME: remove after we fix an issue where basetoken is not set after migration.
            Err(_) => Address::ZERO,
        };
        let st_address = contract
            .getHyperchain(U256::from(chain_id))
            .call()
            .await?
            ._0;

        let stm_contract = IChainTypeManager::new(stm_address, &self.provider);

        let validator_timelock_address = stm_contract
            .validatorTimelock()
            .call()
            .await?
            .validatorTimelock;

        let asset_id = contract
            .ctmAssetIdFromChainId(U256::from(chain_id))
            .call()
            .await?
            ._0;

        Ok(BridgehubChainDetails {
            stm_address,
            st_address,
            base_token_address,
            validator_timelock_address,
            stm_asset_id: asset_id,
        })
    }

    pub async fn get_state_transition(&self, chain_id: u64) -> eyre::Result<StateTransition> {
        let contract = IBridgehub::new(self.address, &self.provider);

        let st_address = contract
            .getHyperchain(U256::from(chain_id))
            .call()
            .await?
            ._0;
        StateTransition::new(&self.provider, st_address).await
    }

    pub async fn get_all_chains_balances(
        &self,
        sequencer: &Sequencer,
    ) -> eyre::Result<HashMap<u64, HashMap<String, U256>>> {
        let mut result = HashMap::new();

        for chain_id in &self.known_chains {
            let foo = self.get_chain_balances(sequencer, *chain_id).await?;
            result.insert(*chain_id, foo);
        }

        Ok(result)
    }

    pub async fn get_chain_balances(
        &self,
        sequencer: &Sequencer,
        chain_id: u64,
    ) -> eyre::Result<HashMap<String, U256>> {
        let mut result = HashMap::new();
        match &self.asset_router {
            AssetRouter::L1(router) => {
                let assets = router.registered_assets.iter().filter_map(|(k, x)| {
                    if let AssetHandler::NativeTokenVault(_) = &x.handler {
                        Some((k, x))
                    } else {
                        None
                    }
                });
                for (asset_id, asset) in assets {
                    let amount = router
                        .chain_balance(sequencer, chain_id.try_into().unwrap(), asset_id)
                        .await;

                    result.insert(asset.name(), amount);
                }
            }

            AssetRouter::L2(_) => eyre::bail!("Not implemented yet"),
        };

        Ok(result)
    }
}
