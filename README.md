# RAEN 🌧

A build tool for NEAR smart contracts.

With RAEN, you can:

* `build`: compile a contract, generate its Application Contract Interface (ACI), and inject it into a custom section of the contract's [WebAssembly][Wasm] (Wasm) binary.
* `fetch` _[coming soon]_: use the ACI of a deployed contract to generate source code bindings for cross contract calls and client interfaces.

NEAR works with any programming language that compiles to Wasm, but the most advanced NEAR SDK and documentation currently exist [for Rust](https://www.near-sdk.io/).

RAEN, too, is built around a [language-agnostic standard][Wit], but currently only works with contracts written in [Rust](https://www.rust-lang.org/).

  [Wasm]: https://webassembly.org/
  [Wit]: https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md


### About the name

"RAEN" is "NEAR" spelled backwards. It is pronounced the same as "rain".


# Install

## Rust

`raen` is a Rust crate and can be installed via `cargo`:

```bash
cargo install raen
```

If you don't yet have Rust, you will need to install it before running the above command. On Linux or MacOS, use the following command:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh

source $HOME/.cargo/env
```

Then, add the `wasm32-unknown-unknown` toolchain. This toolchain is required because NEAR contracts need to be compiled to [Wasm] with _architecture_ and _platform_ both "unknown".

```bash
rustup target add wasm32-unknown-unknown
```

Then follow [these instructions](https://doc.rust-lang.org/book/ch01-01-installation.html) for setting up Rust.

## npm (optionally)

`raen` is also distributed on npm which wraps the rust binary built for your computer:

```bash
npm install --global raen-cli
```

## Additional requirements

`raen` is designed as an extension for [`near-cli-rs`](https://github.com/near/near-cli-rs/tree/master/extensions), but until this project is stable you need to install the JS version `near-cli` to deploy your contracts.

```bash
npm install --global near-cli
```


# Use

For an intro to NEAR, Rust, and RAEN, see **[✨ The Guide ✨](https://raen.dev/guide)**. The basics:

In a NEAR smart contract project:

```bash
raen build --release
```

This builds the contract (or contracts if you're using a Rust workspace), embeds the contract's types in a [Custom Section](https://webassembly.github.io/spec/core/appendix/custom.html), and places them in `./target/res/crate_name.wasm`.

You can then deploy as usual with `near-cli`. With `dev-deploy`:

```bash
near dev-deploy --wasmFile ./target/res/crate_name.wasm
```

With `deploy`:

```bash
near deploy --wasmFile ./target/res/crate_name.wasm --accountId YOUR_ACCOUNT_NAME_HERE
```

Various current and future tooling can now use your contract's types to make your life easier. For example, you can enter your contract's address at [raen.dev/admin](https://raen.dev/admin) for a full admin panel.


# Learn: How it works: Wasm 💖️ Wit

[Wit] is an emerging standard to ease interoperability between programs compiled to WebAssembly, aka [Wasm]. (Wit stands for "WebAssembly Interface Types.") Wit will eventually merge with Wasm, allowing all compiled Wasm modules to explain their interfaces to other Wasm modules and their developers.

NEAR runs Wasm. Any Wasm language, though Rust has the most mature tooling and documentation.

When you build your smart contracts with `raen`, it injects a full Wit specification for your contract into a Wasm [Custom Section](https://webassembly.github.io/spec/core/appendix/custom.html). It compresses it with [brotli](https://www.brotli.org/) to reduce [storage](https://docs.near.org/docs/concepts/storage-staking) requirements.

How much does this increase your contract size? In our tests so far, contracts compiled with `raen` end up **smaller** than before!

`raen` uses `witme` [to generate the Wit](https://ahalabs.dev/posts/wit-bringing-types-to-near-contracts) and inject it into the Wasm Custom Section. Under the hood, `witme` uses [walrus](https://github.com/AhaLabs/wasm-walrus-tools), which optimizes the Wasm enough to reduce the total contract size, even with the Wit. We will add full benchmarks soon.

(Note that currently a JSON [AJV](https://ajv.js.org/) specification is added to a Wasm Custom Section _instead of_ Wit, since 1. No Wit-to-AJV tooling currently exists for browsers, 2. This admin panel relies on AJV, and 3. No other tooling currently makes use of Wit. The AJV custom section will be swapped for a Wit custom section once Wit settles on an [initial syntax version](https://github.com/bytecodealliance/wit-bindgen/issues/214#issuecomment-1116237538).)

This admin panel then reads in that Custom Section, decompresses the brotli, and uses [react-jsonschema-form](https://github.com/rjsf-team/react-jsonschema-form) to allow interacting with the contract.

# Contribute

* Clone this repository
* Build with `cargo build`
* Send us issues or pull requests with questions and we'll improve this section!
