Developer Manual
================

The code is bootstrapped with the [`substrate-node-template`][node-template].

[node-template]: https://github.com/substrate-developer-hub/substrate-node-template

Running development node
------------------------

~~~
BUILD_DUMMY_WASM_BINARY=0 cargo build --release -p radicle_registry_node
./scripts/run-dev-node
~~~

The run script purges the chain data before running to avoid consensus issues.
This means that state is not persisted between runs.

Packages
--------

* `runtime` contains the Substrate runtime code that defines the ledger and
  lives on chain.
* `node` contains the node code which includes the runtime code.
* `client` contains the high-level client library for interacting with the
  registry through a node.
* `subxt` contains a copy of [`subxt`][subxt], the Rust client library for
  substrate. This package serves as the base for `client`.

Upstream `subxt`
----------------

This repository contains a modified copy of [`subxt`][subxt] in the `./subxt`
directory. The repository also contains a Git submodule as reference to the
`subxt` upstream.

To include upstream patches of `subxt` in our copy use the following recipe

~~~bash
# Extract latest patches
git --git-dir=subxt/vendor/.git fetch origin
git --git-dir=subxt/vendor/.git format-patch HEAD..origin/master

# Apply patches
git am --directory=subxt *.patch

# Update submodule revision
git --git-dir=subxt/vendor/.git checkout origin/master
~~~

[subxt]: https://github.com/paritytech/substrate-subxt


Updating substrate
------------------

To update the revision of substrate run
~~~
./scripts/update-substrate REV
~~~
where `REV` is the new Git revision SHA.


Updating Continuous Integration's base Docker image
---------------------------------------------------

After performing the necessary changes to the Dockerfile located in
`ci/base-image/Dockerfile`, move to the root of the `radicle-registry`
repository and run the following:

```bash
docker build ci/base-image --tag gcr.io/opensourcecoin/radicle-registry
docker push gcr.io/opensourcecoin/radicle-registry
```

The `docker push` command outputs the pushed image’s digest. To use the pushed
image in Buildkite runs, update the `DOCKER_IMAGE` value in
`.buildkite/pipeline.yaml` with the new digest.

Note that an account with permission to push to the Google Cloud Registry
address at `gcr.io/opensourcecoin/radicle-registry` is required in order for
these commands to work.
Specifically, you'll need to run
`gsutil iam ch user:<your_monadic_email_address>@monadic.xyz:objectViewer gs://artifacts.opensourcecoin.appspot.com`

For more information on GCR permissions, consult
https://cloud.google.com/container-registry/docs/access-control.

If all this fails, request assistance to someone that can grant these
permissions.