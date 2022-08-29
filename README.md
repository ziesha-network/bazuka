# ℤ - Bazuka!

[![Bazuka](https://github.com/zeeka-network/bazuka/actions/workflows/actions.yml/badge.svg)](https://github.com/zeeka-network/bazuka/actions/workflows/actions.yml)
[![codecov](https://codecov.io/gh/zeeka-network/bazuka/branch/master/graph/badge.svg?token=8XTLET5GQN)](https://codecov.io/gh/zeeka-network/bazuka)

Bazuka is a wallet and node software for the Zeeka (ℤ) Protocol. Zeeka is a novel
layer-1 cryptocurrency which uses Zero-Knowledge proofs as the backend of its
smart-contract (I.e Zero Contracts).

Bazuka ensures the availability of latest contract-states, so that they remain
public and everybody is able to update and build on them, making Zeeka a more
decentralized protocol compared to similar projects.

### Links

 - Website: https://zeeka.io
 - Whitepaper: https://hackmd.io/@keyvank/zeeka
 - Discord: https://discord.gg/4gbf9gZh8H

### How to run a Bazuka node?

**NOTE:** ***Bazuka! is a very early-stage implementation of the Zeeka Protocol,
highly unstable and not yet complete!***

If you only want to run a Zeeka node and do not want to execute Zero Contract or
mine Zeeka, you will only need to install `bazuka` (This repo). In case you also
want to mine Zeeka, you will need to install ![zoro](https://github.com/zeeka-network/zoro)
(The Main Payment Network executor) and also the ![uzi-miner](https://github.com/zeeka-network/uzi-miner)
(A RandomX CPU miner).

**How to install `bazuka`?**

 * Prepare a Linux machine.
 * Make sure you have installed `libssl-dev` and `cmake` packages.
 * Install the Rust toolchain (https://rustup.rs/)
 * Clone the `bazuka` repo: `git clone https://github.com/zeeka-network/bazuka`.
 * Compile and install: `cd bazuka && cargo install --path .`

Now if you want to join the `debug` testnet, you first have to initialize your
node by running:

```sh
bazuka init --seed [your seed phrase] --network debug --node 127.0.0.1:8765
```

Your regular and zero-knowledge private-keys are derived from the `--seed` value,
so **don't forget to keep them somewhere safe**! You can choose your target network
(E.g `mainnet`, `chaos`, `debug` or etc.) through the `--network` option. You should
also choose the node which you want to send your transactions to, through the
`--node` option. (Here we choose our local node which we will run later)

Run your node:

```sh
bazuka node --listen 0.0.0.0:8765 --external [your external ip]:8765 \
  --network debug --db ~/.bazuka-debug --bootstrap [bootstrap node 1] --bootstrap [bootstrap node 2] ...
```

You can use the nodes introduced by the community as your `--bootstrap` nodes.
You either have to run your node on a machine with a static IP, or configure a NAT
Virtual Server in order to expose your node on a public IP. Specify your public IP
through the `--external` option.

### What is the Main Payment Network?

The Main Payment Network (MPN) is a special, builtin smart-contract that is
created in the genesis-block of the Zeeka Protocol. It manages a merkle-tree of
millions of accounts that can transfer ℤ with each other with nearly zero-cost
transactions. It uses the Groth16 proving system and has a fixed number of
transaction slots per update.

People who want to transfer their Zeeka tokens cheaply, would need to deposit
their funds to MPN through the `deposit` command.

### [WIP] How to mine ℤeeka?

In order to be a miner, besides working on a PoW puzzle, you will also need to
execute the MPN contract on each block you generate. The `zoro` software is
a CPU-executor of MPN contract. First install `zoro`, then make sure the
proving-parameters are in the right place and then run:

```sh
zoro --node 127.0.0.1:8765 --seed [seed phrase for the executor account]
```

After a new block is generated, the `uzi-miner` should start working on the PoW
puzzle, so you will also need to have `uzi-miner` running on your system:

```sh
uzi-miner --node 127.0.0.1:8765 --threads 32
```

(Note: Change number of `--threads` based on the spec of your system)
