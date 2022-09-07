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

Highly recommended to also provide your Discord handle through the
`--discord-handle` flag. By providing your handle, you will leave our bots a
way to contact you regarding the problems you may have in your node and its status.

### What is the Main Payment Network?

The Main Payment Network (MPN) is a special, builtin smart-contract that is
created in the genesis-block of the Zeeka Protocol. It manages a merkle-tree of
millions of accounts that can transfer ℤ with each other with nearly zero-cost
transactions. It uses the Groth16 proving system and has a fixed number of
transaction slots per update.

People who want to transfer their Zeeka tokens cheaply, would need to deposit
their funds to MPN through the `deposit` command.

### How to mine ℤeeka?

In order to be a miner, besides working on a PoW puzzle, you will also need to
execute the MPN contract on each block you generate. The `zoro` software is
a CPU-executor of MPN contract. In order to run `zoro`, you'll need a machine
with at least 32GB of RAM. If you want to be competitive you'll also need a
competitive CPU. (In future versions, GPU will be used instead of CPU)

1. Make sure Bazuka is the latest version.

```
cd bazuka
git pull origin master
cargo install --path .
```

If you get a `DifferentGenesis` error, it means that the genesis block has changed
in the updated software. So you need to start fresh. Remove the `~/.bazuka-debug`
folder by running: `rm -rf ~/.bazuka-debug`

Now run Bazuka again.

2. Install the MPN executor (`zoro`)

```
git clone https://github.com/zeeka-network/zoro
cd zoro
cargo install --path .
```

3. Download the proving parameters

- Payment parameters (~700MB):
```
wget --load-cookies /tmp/cookies.txt "https://docs.google.com/uc?export=download&confirm=$(wget --quiet --save-cookies /tmp/cookies.txt --keep-session-cookies --no-check-certificate 'https://docs.google.com/uc?export=download&id=1sR-dJlr4W_A0sk37NkZaZm8UncMxqM-0' -O- | sed -rn 's/.*confirm=([0-9A-Za-z_]+).*/\1\n/p')&id=1sR-dJlr4W_A0sk37NkZaZm8UncMxqM-0" -O ~/bazuka/payment_params.dat && rm -rf /tmp/cookies.txt
```
- Update parameters (~6GB): 
```
wget --load-cookies /tmp/cookies.txt "https://docs.google.com/uc?export=download&confirm=$(wget --quiet --save-cookies /tmp/cookies.txt --keep-session-cookies --no-check-certificate 'https://docs.google.com/uc?export=download&id=149tUhC0oXJxsXDnx7vODkOZtIYzC_5HO' -O- | sed -rn 's/.*confirm=([0-9A-Za-z_]+).*/\1\n/p')&id=149tUhC0oXJxsXDnx7vODkOZtIYzC_5HO" -O ~/bazuka/update_params.dat && rm -rf /tmp/cookies.txt
```
payment_params.dat and update_params.dat are now located in the ~/bazuka directory.

4. Run `zoro` beside your node

```sh
zoro --node 127.0.0.1:8765 --seed [seed phrase for the executor account] --network debug \
  --update-circuit-params [path to update_params.dat] --payment-circuit-params [path to payment_params.dat] \
  --db [absolute path to ~/.bazuka-debug]
```

(Note: The seed phrase for the executor account needs to be different from the
seed you use for your node!)

5. After a new block is generated, the `uzi-miner` should start working on the PoW
puzzle, so you will also need to have `uzi-miner` running on your system:

```
git clone https://github.com/zeeka-network/uzi-miner
cd uzi-miner
cargo install --path .
uzi-miner --node 127.0.0.1:8765 --threads 32
```

(Note: Change number of `--threads` based on the spec of your system)
