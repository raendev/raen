# Build tool for NEAR smart contracts

Currently supports Rust Contracts.

```bash
cargo install raen
```

or npm

```bash
npm i -g raen-cli
```

## Requires rust installation

[rustup](https://rustup.rs/) is recommeneded

```bash
rustup target add wasm32-unknown-unknown
```

## Commands

### `build`

'build' compiles a workspace of contracts and generates wit, ts, and json. The `json` is then compressed and inject into the compiled contract's Wasm file in a custom section named `json`.  This file is placed into `./target/res/<crate_name>.wasm`

This usually results in a smaller binary since we use walrus under the hood.


## Guide

[rean.dev/guide](https://raen.dev/guide)
