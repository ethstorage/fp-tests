# `fp-tests`

A suite of tests for the [fault proof programs][fpp] and [fault proof VMs][fpvm] of the [OP Stack][op-stack].

## Usage
 
The `fpt` binary in this repository is the basis for both generating and running test cases. It handles:
* Downloading and building relevant versions of the software in the `registry.toml`
* The creation of the test cases found in `tests`
* Running the test cases on the matrix of available platforms / programs.

At the root, the `registry.toml` defines the available platforms and programs that the runner has available.

### `fpt`

_TODO_

[op-stack]: https://docs.optimism.io
[fpp]: https://specs.optimism.io/fault-proof/index.html 
[fpvm]: https://specs.optimism.io/fault-proof/cannon-fault-proof-vm.html 
