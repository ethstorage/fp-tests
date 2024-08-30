# default recipe to display help information
default:
  @just --list

# Test for the native target with all features
test *args='':
  cargo nextest run --workspace --all --all-features $@

# Fixes the formatting of the workspace
fmt-fix:
  cargo +nightly fmt --all

# Check the formatting of the workspace
fmt-check:
  cargo +nightly fmt --all -- --check

# Lint the workspace
lint: fmt-check
  cargo +nightly clippy --workspace --all --all-features --all-targets -- -D warnings

# Lint the Rust documentation
lint-docs:
  RUSTDOCFLAGS="-D warnings" cargo doc --all --no-deps --document-private-items 

# Test the Rust documentation
test-docs:
  cargo test --doc --all --locked

# Build the workspace
build:
  cargo build --workspace --all $@

# Install the devnet
install-devnet:
  #!/bin/bash

  if [ -d "./devnet" ]; then
    exit 0
  fi

  git clone https://github.com/ethpandaops/optimism-package && mv optimism-package devnet

  T8N_NETWORK_PARAMS=$(cat <<- "EOM"
  optimism_package:
    participants:
      - el_type: op-geth
        cl_type: op-node
    network_params:
      seconds_per_slot: 2
      network_id: 1337
  ethereum_package:
    participants:
      - el_type: reth
        cl_type: lighthouse
    network_params:
      preset: minimal
  EOM
  )
  printf "%s" "$T8N_NETWORK_PARAMS" > ./devnet/network_params.yaml

# Start the devnet
start-devnet:
  #!/bin/bash

  SCRIPT_DIR=$( pwd )
  KURTOSIS_DIR="$SCRIPT_DIR/devnet"

  # Exit if Kurtosis is already running
  kurtosis enclave inspect devnet && exit 0

  echo "Starting Kurtosis network..."
  cd "$KURTOSIS_DIR" || exit 1
  kurtosis clean -a
  kurtosis run --enclave devnet . --args-file ./network_params.yaml

  echo "Returning to opt8n..."
  cd "$SCRIPT_DIR" || exit 1

# Stop the devnet
stop-devnet:
  #!/bin/bash
  kurtosis clean -a

# View important ports for the local devnet instance
devnet-ports:
  #!/bin/bash
  L1_EL_PORT=$(kurtosis enclave inspect devnet | grep 'el-1-reth-lighthouse' -A5 | grep " rpc:" | awk -F ' -> ' '{print $2}' | awk -F ':' '{print $2}' | tr -d ' \n\r')
  L1_BEACON_PORT=$(kurtosis enclave inspect devnet | grep 'cl-1-lighthouse-reth' -A5 | grep " http: " | awk -F ' -> ' '{print $2}' | awk -F ':' '{print $3}' | awk -F ' ' '{print $1}' | tr -d ' \n\r')
  L2_EL_PORT=$(kurtosis enclave inspect devnet | grep 'op-el-1-op-geth-op-node' -A5 | grep " rpc:" | awk -F ' -> ' '{print $2}' | awk -F ':' '{print $3}' | tr -d ' \n\r')
  L2_NODE_PORT=$(kurtosis enclave inspect devnet | grep 'op-cl-1-op-node-op-geth' -A5 | grep " http: " | awk -F ' -> ' '{print $2}' | awk -F ':' '{print $3}' | awk -F ' ' '{print $1}' | tr -d ' \n\r')

  echo "L1 EL Port: $L1_EL_PORT"
  echo "L1 Beacon Port: $L1_BEACON_PORT"
  echo "L2 EL Port: $L2_EL_PORT"
  echo "L2 Node Port: $L2_NODE_PORT"
