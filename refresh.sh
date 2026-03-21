L1_URL_FLAG=""
if [ -n "$L1_URL" ]; then
  L1_URL_FLAG="--l1-url $L1_URL"
fi

cargo run -- --no-gateway --network mainnet $L1_URL_FLAG --output web/public/output.mainnet.json
cargo run -- --no-gateway --network testnet --output web/public/output.testnet.json
cargo run -- --no-gateway --network testnet-atlas --output web/public/output.testnet_atlas.json
