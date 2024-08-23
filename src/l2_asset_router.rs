use std::fmt::Display;

use alloy::{primitives::Address, sol};

use crate::sequencer::Sequencer;

sol! {
    #[sol(rpc)]
    contract IL2AssetRouter {

            function l1AssetRouter() external view returns (address);


    }
}

// a.k.a SharedBridge
pub struct L2AssetRouter {
    pub address: Address,
    pub l1_router: Address,
}
impl Display for L2AssetRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== L2 Asset -  @ {}  ", self.address)?;
        writeln!(f, "   L1 router:   {}", self.l1_router)?;

        Ok(())
    }
}

impl L2AssetRouter {
    pub async fn new(sequencer: &Sequencer, address: Address) -> Self {
        let provider = sequencer.get_provider();
        let contract = IL2AssetRouter::new(address, provider);

        let l1_router = contract.l1AssetRouter().call().await.unwrap()._0;

        Self { address, l1_router }
    }
}
