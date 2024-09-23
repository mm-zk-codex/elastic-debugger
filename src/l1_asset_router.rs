use std::{collections::HashMap, fmt::Display};

use alloy::{
    primitives::{address, Address, FixedBytes, U256},
    sol,
    sol_types::SolEvent,
};

use futures::future::join_all;

use crate::{
    sequencer::Sequencer,
    utils::{address_from_fixedbytes, get_all_events, get_human_name_for},
};

use colored::Colorize;

sol! {
    #[sol(rpc)]
    contract IL1AssetRouter {

        function nativeTokenVault() external view returns(address);
        function BRIDGE_HUB() external view returns(address);

        event AssetHandlerRegisteredInitial(
            bytes32 indexed assetId,
            address indexed assetHandlerAddress,
            bytes32 indexed additionalData,
            address sender
        );
    }

    #[sol(rpc)]
    contract NativeTokenVault {
        function tokenAddress(bytes32) external view returns(address);
        function getERC20Getters(address _token) external view returns (bytes memory);
        function chainBalance(uint256 _chainId, bytes32 assetId) external view returns (uint256);

    }
    #[sol(rpc)]
    contract ERC20 {
        function name() external view returns(string);

    }
}

pub struct RegisteredAsset {
    pub asset_id: FixedBytes<32>,
    pub handler: AssetHandler,
}

#[derive(Debug)]
pub struct NativeTokenVaultAsset {
    pub address: Address,
    pub token_name: String,
}

#[derive(Debug)]
pub enum AssetHandler {
    Bridgehub,
    NativeTokenVault(NativeTokenVaultAsset),
    Other(Address),
}

impl AssetHandler {
    pub fn short_fmt(&self) -> String {
        match self {
            AssetHandler::Bridgehub => "Bridgehub (STM)".to_owned(),
            AssetHandler::NativeTokenVault(ntv_asset) => {
                format!("Token {} {}", ntv_asset.token_name, ntv_asset.address)
            }
            AssetHandler::Other(address) => format!("Uknown {}", address),
        }
    }
}

impl RegisteredAsset {
    pub async fn new(
        sequencer: &Sequencer,
        asset_id: FixedBytes<32>,
        deployment_tracker: Address,
        native_token_vault: &Address,
        bridgehub: &Address,
    ) -> Self {
        let provider = sequencer.get_provider();
        let native_token_vault_contract =
            NativeTokenVault::new(native_token_vault.clone(), provider);

        let handler = match deployment_tracker {
            ref dt if dt == native_token_vault => {
                let token_address = native_token_vault_contract
                    .tokenAddress(asset_id)
                    .call()
                    .await
                    .unwrap()
                    ._0;

                let token_name =
                    if token_address == address!("0000000000000000000000000000000000000001") {
                        "ETH".to_owned()
                    } else {
                        let erc20_contract = ERC20::new(token_address, sequencer.get_provider());
                        erc20_contract.name().call().await.unwrap()._0
                    };

                AssetHandler::NativeTokenVault(NativeTokenVaultAsset {
                    address: token_address,
                    token_name,
                })
            }

            ref dt if dt == bridgehub => AssetHandler::Bridgehub,
            _ => AssetHandler::Other(deployment_tracker),
        };
        Self {
            asset_id,
            handler: handler,
        }
    }

    pub fn name(&self) -> String {
        match &self.handler {
            AssetHandler::Bridgehub => get_human_name_for(self.asset_id),
            AssetHandler::NativeTokenVault(vault_asset) => format!(
                "{}-{}",
                vault_asset.token_name,
                get_human_name_for(self.asset_id)
            ),
            AssetHandler::Other(_) => get_human_name_for(self.asset_id),
        }
    }

    pub fn detailed_fmt(&self, f: &mut std::fmt::Formatter<'_>, pad: usize) -> std::fmt::Result {
        let pad = " ".repeat(pad);
        writeln!(f, "{}Asset:     {}", pad, self.name().bold())?;
        writeln!(f, "{}  id:      {}", pad, self.asset_id)?;
        writeln!(f, "{}  tracker: {}", pad, self.handler.short_fmt())?;

        Ok(())
    }
}

impl Display for RegisteredAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.detailed_fmt(f, 0)
    }
}

// a.k.a SharedBridge
pub struct L1AssetRouter {
    pub address: Address,
    pub native_token_vault: Address,
    pub registered_assets: HashMap<FixedBytes<32>, RegisteredAsset>,
}
impl Display for L1AssetRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.detailed_fmt(f, 0)
    }
}

impl L1AssetRouter {
    pub async fn new(sequencer: &Sequencer, address: Address) -> eyre::Result<Self> {
        let provider = sequencer.get_provider();
        let contract = IL1AssetRouter::new(address, provider);

        let native_token_vault = contract.nativeTokenVault().call().await?._0;
        let bridgehub = contract.BRIDGE_HUB().call().await.unwrap()._0;

        let registered_assets = get_all_events(
            sequencer,
            address,
            IL1AssetRouter::AssetHandlerRegisteredInitial::SIGNATURE_HASH,
        )
        .await
        .unwrap()
        .into_iter()
        .map(|log| {
            RegisteredAsset::new(
                sequencer,
                // Asset Id
                log.topics().get(1).unwrap().clone(),
                // Address for handler
                address_from_fixedbytes(log.topics().get(2).unwrap()).unwrap(),
                &native_token_vault,
                &bridgehub,
            )
        });

        let registered_assets = join_all(registered_assets)
            .await
            .into_iter()
            .map(|elem| (elem.asset_id, elem));

        Ok(Self {
            address,
            native_token_vault,
            registered_assets: HashMap::from_iter(registered_assets),
        })
    }

    pub async fn chain_balance(
        &self,
        sequencer: &Sequencer,
        chain_id: U256,
        asset_id: &FixedBytes<32>,
    ) -> U256 {
        let provider = sequencer.get_provider();
        let contract = NativeTokenVault::new(self.native_token_vault, provider);
        let balance = contract
            .chainBalance(chain_id, *asset_id)
            .call()
            .await
            .unwrap()
            ._0;

        balance
    }

    pub fn detailed_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        pad_size: usize,
    ) -> std::fmt::Result {
        let pad = " ".repeat(pad_size);
        writeln!(f, "{}=== L1 Asset Router -  {}  ", pad, self.address)?;
        writeln!(
            f,
            "{}Native vault:          {}",
            pad, self.native_token_vault
        )?;
        writeln!(f, "{}Assets: ", pad)?;
        for v in self.registered_assets.values() {
            v.detailed_fmt(f, pad_size + 3)?;
        }

        Ok(())
    }
}
