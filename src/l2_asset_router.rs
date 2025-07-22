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
    //pub l1_router: Address,
}
impl Display for L2AssetRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.detailed_fmt(f, 0)
    }
}

impl L2AssetRouter {
    pub async fn new(sequencer: &Sequencer, address: Address) -> Self {
        let provider = sequencer.get_provider();
        let contract = IL2AssetRouter::new(address, provider);

        //let l1_router = contract.l1AssetRouter().call().await.unwrap()._0;

        Self { address }
    }
    pub fn detailed_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        pad_size: usize,
    ) -> std::fmt::Result {
        let pad = " ".repeat(pad_size);
        writeln!(f, "{}=== L2 Asset -  {}  ", pad, self.address)?;
        //writeln!(f, "{}   L1 router:   {}", pad, self.l1_router)?;
        Ok(())
    }
}
