use std::{fmt::Display, net::TcpStream, time::Duration};

use alloy::{
    hex::FromHex,
    primitives::Address,
    providers::{Provider, ProviderBuilder, RootProvider},
    transports::http::{reqwest::Response, Client, Http},
};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_json::json;

pub const REDACTED_RPC_URL: &str = "[redacted]";

#[derive(Clone)]
pub struct Sequencer {
    pub rpc_url: String,
    pub chain_id: u64,
    pub latest_block: u64,
    pub sequencer_type: SequencerType,
}

#[derive(Clone, Debug, Serialize)]
pub enum SequencerType {
    L1,
    L2(L2SequencerInfo),
}

#[derive(Clone, Debug, Serialize)]
pub struct L2SequencerInfo {
    pub l1_chain_id: u64,
    pub bridgehub_address: Address,
}

impl Display for Sequencer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sequencer_type_short = match &self.sequencer_type {
            SequencerType::L1 => "L1".to_string(),
            SequencerType::L2(info) => format!("L2 -> {}", info.l1_chain_id),
        };
        write!(
            f,
            "Sequencer at {} (Chain: {}, Last Block: {}), {}",
            self.rpc_url, self.chain_id, self.latest_block, sequencer_type_short
        )
    }
}

impl Sequencer {
    pub fn output_rpc_url(&self) -> &str {
        match self.sequencer_type {
            SequencerType::L1 => REDACTED_RPC_URL,
            SequencerType::L2(_) => &self.rpc_url,
        }
    }

    pub fn get_provider(&self) -> RootProvider<Http<Client>> {
        let provider: alloy::providers::RootProvider<
            alloy::transports::http::Http<alloy::transports::http::Client>,
        > = ProviderBuilder::new().on_http(self.rpc_url.parse().unwrap());

        provider
    }
}

fn is_port_active(address: &str) -> bool {
    if address.starts_with("https:/") {
        // Assume that https urls are always active.
        return true;
    }

    let timeout = Duration::from_secs(1);
    let address = address.strip_prefix("http://").or(Some(address)).unwrap();

    match TcpStream::connect_timeout(&address.parse().unwrap(), timeout) {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn send_json_request(url: &str, method: &str) -> eyre::Result<Response> {
    let client = Client::new();

    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": []
    });

    let response: Response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    Ok(response)
}

#[derive(Deserialize, Debug)]
struct BridgehubResult {
    result: String,
}

async fn get_bridgehub_address(url: &str) -> eyre::Result<Address> {
    let response = send_json_request(url, "zks_getBridgehubContract").await?;
    let response_json: BridgehubResult = response.json().await?;
    Ok(Address::from_hex(response_json.result)?)
}

#[derive(Deserialize, Debug)]
struct L1ChainIdResult {
    result: String,
}

async fn get_l1_chain_id(url: &str) -> eyre::Result<u64> {
    if url == "http://127.0.0.1:3050" {
        // TODO: small hack for the zksync os server.
        // As it doesnt' expose zks_L1ChainId anymore.
        dbg!("Returning chainid");
        return Ok(31337);
    }

    let response = send_json_request(url, "zks_L1ChainId").await?;
    let response_json: L1ChainIdResult = response.json().await?;
    let trimmed_hex = response_json.result.trim_start_matches("0x");
    Ok(u64::from_str_radix(trimmed_hex, 16)?)
}

// Detects the sequencer that is operating a given host / port.
// Can detect both L1 and L2.
pub async fn detect_sequencer(rpc_url: &str) -> eyre::Result<Sequencer> {
    if !is_port_active(rpc_url) {
        eyre::bail!("Port not active: {}", rpc_url);
    }

    // Create a provider with the HTTP transport using the `reqwest` crate.
    let provider: alloy::providers::RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
    > = ProviderBuilder::new().on_http(rpc_url.parse()?);

    let chain_id = provider.get_chain_id().await?;
    println!("Detected chain ID: {}", chain_id);
    let latest_block = provider.get_block_number().await?;

    // Now let's see if this is an 'L2' or 'L1'.
    let sequencer_type = match get_bridgehub_address(rpc_url).await {
        Ok(bridgehub_address) => SequencerType::L2(L2SequencerInfo {
            bridgehub_address,
            // FIXME: default to 13 if we can't get it?
            l1_chain_id: get_l1_chain_id(rpc_url).await.unwrap_or(13),
        }),
        Err(_) => SequencerType::L1,
    };

    println!(
        "Detected sequencer type: {:?} at {}",
        sequencer_type, rpc_url
    );

    Ok(Sequencer {
        rpc_url: rpc_url.to_string(),
        chain_id,
        latest_block,
        sequencer_type,
    })
}

impl Serialize for Sequencer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Sequencer", 4)?;
        state.serialize_field("rpc_url", self.output_rpc_url())?;
        state.serialize_field("chain_id", &self.chain_id)?;
        state.serialize_field("latest_block", &self.latest_block)?;
        state.serialize_field("sequencer_type", &self.sequencer_type)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    #[test]
    fn serializes_l1_rpc_url_as_redacted() {
        let sequencer = Sequencer {
            rpc_url: "https://sensitive.example".to_string(),
            chain_id: 1,
            latest_block: 42,
            sequencer_type: SequencerType::L1,
        };

        let value = serde_json::to_value(&sequencer).unwrap();

        assert_eq!(value["rpc_url"], REDACTED_RPC_URL);
    }

    #[test]
    fn keeps_l2_rpc_url_in_serialized_output() {
        let sequencer = Sequencer {
            rpc_url: "https://l2.example".to_string(),
            chain_id: 270,
            latest_block: 42,
            sequencer_type: SequencerType::L2(L2SequencerInfo {
                l1_chain_id: 1,
                bridgehub_address: address!("0000000000000000000000000000000000000001"),
            }),
        };

        let value = serde_json::to_value(&sequencer).unwrap();

        assert_eq!(value["rpc_url"], "https://l2.example");
    }
}
