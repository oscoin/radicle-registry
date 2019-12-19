Radicle Registry
================

Experimental Radicle Registry implementation with Substrate.

See [`DEVELOPING.md`][dev-manual] for developer information.

<!-- toc -->

- [Using the Client](#using-the-client)
- [Getting and Running the Node](#getting-and-running-the-node)
- [Account Keys](#account-keys)
- [Developing with the Client](#developing-with-the-client)
- [Building and running the node](#building-and-running-the-node)
- [Using the CLI](#using-the-cli)
- [Build Artifacts](#build-artifacts)
- [Registry Specification](#registry-specification)

<!-- tocstop -->

Using the Client
----------------

The client for the registry node is provided by the `radicle-registry-client`
package in the `./client` directory.

You’ll need to build the client with Rust Nightly.

To build and view the client documentation run `./scripts/build-client-docs
--open`.

You can find examples in the `./client/examples` directory.


Getting and Running the Node
----------------------------

We build binaries of the node and docker images for every master commit.

The node binaries are available for the `x86_64-uknown-linux-gnu` target triple.
They are provided as artifacts by our CI. You can find the through the
[Buildkite UI][buildkite-master].

You can pull a docker image of the node with
```bash
docker pull gcr.io/opensourcecoin/radicle-registry/node:<commit-sha>
```
In the image the node binary is located at `/usr/local/bin/radicle-registry-node`

[buildkite-master]: https://buildkite.com/monadic/radicle-registry/builds?branch=master


Account Keys
------------

We use Ed25519 keys for accounts. Key creation and handling functionality is
provided by the `radicle-registry-client::ed25519` module. To use this module
you will likely need to import the `radicle-registry-client::CryptoPair` and
`radicle-registry-client::CryptoPublic` traits.

You can create key pairs using [`CryptoPair::generate()`][api-pair-generate]
```rust
use radicle-registry-client::{ed25519, CryptoPair};
let (key, seed) = ed25519::Pair::generate();
```

To create keys from human readable strings, use [`CryptoPair::from_string`][api-pair-from-string].
```rust
use radicle-registry-client::{ed25519, CryptoPair};
let alice = ed25519::Pair::from_string("//Alice", None);
```

The `radicle-registry-client::ed25519` module and the crypto traits are
re-exports from [`substrate_primitives::ed25519`][api-ed25519] and
[`substrate_primitives::crypto`][api-crypto], respectively

[api-ed25519]: https://crates.parity.io/substrate_primitives/ed25519/index.html
[api-crypto]: https://crates.parity.io/substrate_primitives/crypto/index.html
[api-pair-generate]: https://crates.parity.io/substrate_primitives/crypto/trait.Pair.html#method.generate
[api-pair-from-string]: https://crates.parity.io/substrate_primitives/crypto/trait.Pair.html#method.from_string


Developing with the Client
--------------------------

For development you can create a ledger emulator with
`radicle-registry-client::Client::new_emulator()`. Instead of connecting to a
node this client runs the ledger in memory. See the API docs for more
information.


Building and running the node
-----------------------------

Building the development node:

1. Get [`rustup`][rustup-install]
2. Run `./scripts/rustup-setup` to install all required `rustup` components and
   targets.
3. Install [`wasm-gc`][wasm-gc] via `cargo install --git https://github.com/alexcrichton/wasm-gc`
4. Build the node with `./scripts/build-dev-node`

You can run the node with

~~~
./scripts/run-dev-node
~~~

To reset the chain state run

~~~
./scripts/run-dev-node purge-chain
~~~

Note that `build-dev-node` will also reset the chain state.

Using the CLI
-------------

We provide a CLI to talk read and update the ledger in the `cli` directory. To
learn more run `cargo run -p radicle-registry-cli -- --help`.


[dev-manual]: ./DEVELOPING.md
[rustup-install]: https://github.com/rust-lang/rustup.rs#installation
[wasm-gc]: https://github.com/alexcrichton/wasm-gc

Build Artifacts
---------------

Each commit's successful CI build uploads an artifact - the built Substrate
node.
In order to access it, you can either use the Buildkite web UI, accessible
via the commit's GitHub status , or run

~~~
curl -H "Authorization: Bearer $TOKEN" https://api.buildkite.com/v2/organizations/monadic/pipelines/radicle-registry/builds/{build.number}/jobs/{job.id}/artifacts/{artifact.id}/download
~~~

where `$TOKEN` is a valid Buildkite API access token that has at least the
`read_artifacts` scope, and replace the items between curly brackets
(e.g. `{build.number}`) with the appropriate values for the build and artifact
that is wanted.

The above command will return a JSON response with a URL to a Buildkite AWS
S3 bucket that contains the desired artifact, which can then be `curl`ed
again.

For more information on Buildkite API tokens and permissions, as well as
access to build artifacts, see:

* https://buildkite.com/docs/apis/rest-api#authentication
* https://buildkite.com/docs/apis/rest-api/artifacts#download-an-artifact

Registry Specification
--------------------

In the `registry-spec` folder, there is a Rust crate that details the Oscoin
registry specification with traits, types and a sizable amount of documentation.

It is intended to bridge the formal description of the registry from the
whitepaper with the registry's future implementation, providing a "sandbox"
with which to test and discuss design ideas before implementing them in
earnest.

The `registry-spec` crate is meant to evolve with the project, and at each point
in time its contents will reflect the team's requirements from and
understanding of the Oscoin registry.

Note that although there is no actual implementation of any function or
datatype in the crate, it compiles and is part of the build process.

### Structure

`registry-spec` is a library with three modules:
* `lib.rs`, defining the main traits with which to interact with the Oscoin
  registry
* `error.rs` defining errors that may arise when interacting with the registry.
* `types.rs`, defining the primitive types that will populate the registry state.

License
-------

This code is licensed under [GPL v3.0](./LICENSE.md).
