# ℤ - Bazuka!

[![Bazuka](https://github.com/zeeka-network/bazuka/actions/workflows/actions.yml/badge.svg)](https://github.com/zeeka-network/bazuka/actions/workflows/actions.yml)
[![codecov](https://codecov.io/gh/zeeka-network/bazuka/branch/master/graph/badge.svg?token=8XTLET5GQN)](https://codecov.io/gh/zeeka-network/bazuka)

Rust implementation of the Zeeka Network

### What is Zeeka?

In simplest words, Zeeka (ℤ) is a cryptocurrency which aims to provide a light and scalable blockchain by extensively using the help of Zero-Knowledge proof technology.

Here you will find a WIP implementation of Zeeka protocol in Rust.

### Links

 - Website: https://zeeka.io
 - Whitepaper: https://hackmd.io/@keyvank/zeeka
 - Discord: https://discord.gg/4gbf9gZh8H

### Quick start

Bazuka! is a very early-stage implementation of Zeeka Protocol in Rust and not
yet complete. But if you are curious enough, you can run it like this:

```
cargo run -- --listen 0.0.0.0:8080 --external [your_public_ip]:8080
```

### Requirements

Install `libssl-dev` and `cmake` packages before compiling.
