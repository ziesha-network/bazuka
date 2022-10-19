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

 - Website: https://zeeka.network
 - Twitter: https://twitter.com/ZeekaNetwork
 - Whitepaper: http://hackmd.io/@geusebetel/zeeka
 - Discord: https://discord.gg/4gbf9gZh8H

### How to run a Bazuka node?

If you only want to run a Zeeka node and do not want to execute Zero Contract or
mine Zeeka, you will only need to install `bazuka` (This repo). In case you also
want to mine Zeeka, you will need to install ![zoro](https://github.com/zeeka-network/zoro)
(The Main Payment Network executor) and also the ![uzi-miner](https://github.com/zeeka-network/uzi-miner)
(A RandomX CPU miner).

**How to install `bazuka`?**

 * Prepare a Linux machine.
 * Make sure you have installed `libssl-dev` and `cmake` packages.
    ```
    sudo apt install -y libssl-dev cmake
    ```
 * Install the Rust toolchain (https://rustup.rs/)
    ```
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
 * Clone the `bazuka` repo:
    ```
    git clone https://github.com/zeeka-network/bazuka
    ```
 * ***Warning:*** Make sure Rust binaries are present in your PATH before compiling:
    ```
    source "$HOME/.cargo/env"
    ```
 * Compile and install:
    ```
    cd bazuka
    cargo install --path .
    ```

Now if you want to join the `chaos` testnet, you first have to initialize your
node. If you have already initialized bazuka for the Debug Testnet, you first need
to remove your previous initialization by running:

```sh
rm ~/.bazuka.yaml
```

Then initialize:

```sh
bazuka init --seed [your seed phrase] --network chaos --node 127.0.0.1:8765
```

(Example seed: `"A_RANDOM_STRING_THAT_IS_LONG_ENOUGH_783264dfhejfjcgefjkef"`)

The seed is a random string of characters, not mnemonic phrases. **You do not need to generate it using wallets! (E.g Metamask)**
The longer the seed is, the safer. We suggest a seed string of at least 80 random characters. Your regular and zero-knowledge private-keys are derived from the `--seed` value, so **don't forget to keep them somewhere safe and do not share it**!

Run your node:

```sh
bazuka node --listen 0.0.0.0:8765 --external [your external ip]:8765 \
  --network chaos --db ~/.bazuka-chaos --bootstrap [bootstrap node 1] --bootstrap [bootstrap node 2] ...
```

You can use the nodes introduced by the community as your `--bootstrap` nodes.
You either have to run your node on a machine with a static IP, or configure a NAT
Virtual Server in order to expose your node on a public IP. Specify your public IP
through the `--external` option.

Highly recommended to also provide your Discord handle through the
`--discord-handle` flag. By providing your handle, you will leave our bots a
way to contact you regarding the problems you may have in your node and its status.

### Useful commands:

`bazuka deposit`     Deposit funds to a Zero-Contract

`bazuka help`       Prints this message or the help of the given subcommand(s)

`bazuka init`        Initialize node/wallet

`bazuka node`        Run node

`bazuka rsend`      Send funds through a regular-transaction

`bazuka status`     Get status of a node

`bazuka withdraw`    Withdraw funds from a Zero-Contract

`bazuka zsend`       Send funds through a zero-transaction
