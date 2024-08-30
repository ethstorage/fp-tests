# `fp-tests`

A suite of tests for the [fault proof programs][fpp] and [fault proof VMs][fpvm] of the [OP Stack][op-stack].

## Overview

* [`src`](./src/) - The source for the `fpt` binary.
* [`tests`](./tests/) - The test fixtures for the fault proof programs.

## `fpt`
 
The `fpt` binary in this repository is the basis for both generating and running test cases. It handles:
* Downloading and building relevant versions of the software in the `registry.toml`
* The creation of the test cases found in `tests`
* Running the test cases on the matrix of available platforms / programs.

At the root, the `registry.toml` defines the available platforms and programs that the runner has available.

### Test Generation

Before generating test cases, install the local devnet with `just install-devnet` and start it up with `just start-devnet`.

To craft the L1 + L2 chain that is being fault proven, interact with the devnet. For convenience, `just devnet-ports` 
returns the ports of the active L1/L2 clients needed for `fpt generate`. A more verbose view of devnet services can be 
found with `kurtosis enclave inspect devnet`.

`fpt generate`, by default, only needs the name of the test fixture to generate, devnet node RPCs, and the block number
of the claimed output root. Other inputs are optional, and if not provided, will be fetched from the devnet remotes.

```sh
Options:
  -n, --name <NAME>
          The name of the test case
      --l1-rpc <L1_RPC>
          The L1 RPC [env: L1_RPC=]
      --l1-beacon-rpc <L1_BEACON_RPC>
          The L1 beacon RPC [env: L1_BEACON_RPC=]
      --l2-node-rpc <L2_NODE_RPC>
          The L2 rollup node RPC [env: L2_NODE_RPC=]
      --l2-rpc <L2_RPC>
          The L2 RPC [env: L2_RPC=]
      --l2-block <L2_BLOCK>
          The L2 block number that the test case is for [env: L2_BLOCK=]
      --l2-claim <L2_CLAIM>
          The L2 claim [env: L2_CLAIM=]
      --l2-output-root <L2_OUTPUT_ROOT>
          The starting L2 output root [env: L2_OUTPUT_ROOT=]
      --l2-head <L2_HEAD>
          The starting L2 head hash [env: L2_HEAD=]
      --l1-head <L1_HEAD>
          The L1 block at the creation of the dispute [env: L1_HEAD=]
      --l2-chain-id <L2_CHAIN_ID>
          The L2 chain ID [env: L2_CHAIN_ID=]
  -h, --help
          Print help
```

### Running Tests

The test runner facilitates executing the [`tests`](./tests) against a matrix of available [FPVMs][fpp] and [FPPs][fpp]
defined in [`registry.toml`](./registry.toml).

```sh
Options:
  -t, --test <TEST>            The test to run (glob pattern supported)
  -v, --vm <VM>                The FPVM to run the tests on
  -p, --program <PROGRAM>      The FPP to run the tests on
      --partition <PARTITION>  The partition of tests to run (e.g 1/4)
      --workers <WORKERS>      The number of active workers [default: 4]
  -h, --help                   Print help
```

[op-stack]: https://docs.optimism.io
[fpp]: https://specs.optimism.io/fault-proof/index.html 
[fpvm]: https://specs.optimism.io/fault-proof/cannon-fault-proof-vm.html 
