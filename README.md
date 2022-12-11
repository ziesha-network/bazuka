# ℤ - Bazuka!

[![Bazuka](https://github.com/ziesha-network/bazuka/actions/workflows/actions.yml/badge.svg)](https://github.com/ziesha-network/bazuka/actions/workflows/actions.yml)
[![codecov](https://codecov.io/gh/ziesha-network/bazuka/branch/master/graph/badge.svg?token=8XTLET5GQN)](https://codecov.io/gh/ziesha-network/bazuka)

Bazuka is a wallet and node software for the Ziesha (ℤ) Protocol. Ziesha is a novel
layer-1 cryptocurrency which uses Zero-Knowledge proofs as the backend of its
smart-contract (I.e Zero Contracts).

Bazuka ensures the availability of latest contract-states, so that they remain
public and everybody is able to update and build on them, making Ziesha a more
decentralized protocol compared to similar projects.

### Links

 - Website: https://ziesha.network
 - Twitter: https://twitter.com/ZieshaNetwork
 - Whitepaper: http://hackmd.io/@geusebetel/ziesha
 - Discord: https://discord.gg/4gbf9gZh8H

### How to run a Bazuka node?

If you only want to run a Ziesha node and do not want to execute Zero Contract or
mine Ziesha, you will only need to install `bazuka` (This repo). In case you also
want to mine Ziesha, you will need to install ![zoro](https://github.com/ziesha-network/zoro)
(The Main Payment Network executor) and also the ![uzi-miner](https://github.com/ziesha-network/uzi-miner)
(A RandomX CPU miner).

**How to install `bazuka`?**

 * Prepare a Linux machine.
 * Make sure you have installed `libssl-dev` and `cmake` packages.
    ```
    sudo apt install -y build-essential libssl-dev cmake
    ```
 * Install the Rust toolchain (https://rustup.rs/)
    ```
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
 * Clone the `bazuka` repo:
    ```
    git clone https://github.com/ziesha-network/bazuka
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

Now if you want to join the `groth-3` testnet, you first have to initialize your
node. If you have already initialized bazuka for the Debug Testnet, you first need
to remove your previous initialization by running:

```sh
rm ~/.bazuka.yaml
```

Then initialize:

```sh
bazuka init [flags...]
```

Available flags:

 * `--bootstrap <bootstrap>...`: You can use the nodes introduced by the community as your bootstrap nodes through this flag.
 * `--db <db>`: Path of the node's database. Default: `~/.bazuka`.
 * `--external <external>`: Public ip/port of your node. Default: `YOUR_PUBLIC_IP:8765`.
 * `--listen <listen>`: Local socket. Default: `0.0.0.0:8765`.
 * `--mnemonic <mnemonic>`: If you already have a 12-word mnemonic phrase, you can pass it through this flag. If not provided, a new wallet will be generated for you. Keep the mnemonic word list somewhere safe!
 * `--network <network>`: The network your node will operate on. Default: `mainnet`

Example to initialize a node with 2 bootstrap nodes `23.34.12.45:8765` and `34.56.78.23:8765` on the `groth-3` network:

```
bazuka init --network groth-6 --bootstrap 23.34.12.45:8765 --bootstrap 34.56.78.23:8765
```

After initializing your node you can run it through:

```sh
bazuka node start --discord_handle "YOUR DISCORD HANDLE"
```

Highly recommended to also provide your Discord handle through the
`--discord-handle` flag. By providing your handle, you will leave our bots a
way to contact you regarding the problems you may have in your node and its status.

### Useful commands:

`bazuka wallet info` Show your wallet address/balances

`bazuka wallet deposit` Deposit funds to the MPN-contract

`bazuka withdraw` Withdraw funds from the MPN-contract

`bazuka wallet rsend` Send funds through a regular-transaction

`bazuka wallet zsend` Send funds through a zero-transaction

`bazuka init` Initialize node/wallet

`bazuka node start` Start your node

`bazuka node status` Get status of a node
