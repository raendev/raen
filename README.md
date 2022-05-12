# Build tool for NEAR smart contracts

Currently supports Rust Contracts.

```bash
cargo install raen
```

### rust installation

[rustup](https://rustup.rs/)

```bash
rustup target add wasm32-unknown-unknown
```

## Commands

### `build`

'build' compiles a workspace of contracts and generates wit, ts, and json. The `json` is then compressed and inject into the compiled contract's Wasm file in a custom section named `json`.

This usually results in a smaller binary since we use walrus under the hood.
