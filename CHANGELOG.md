Change Log
==========

Upcoming
--------

### Breaking changes

* cli: Binary update required to connect to the on-chain runtime with `spec_version` 6,
  no longer including the `CheckVersion` signed extension.
* client: `Client::new_emulator()` now returns a pair of a `Client` and
  `EmulatorControl`.
* cli: Move `update-runtime` to `runtime update`
* Rename `TransactionApplied` to `TransactionIncluded`
* cli: Rename the key-pair storage file from 'accounts.json' to 'key-pairs.json'
* cli: Move key-pair related commands under the new `key-pair` command group
* client: `TransactionApplied` result is now `Result<(), TransactionError>`
  for all messages.
* cli: Drop `_TX_` from the environment options
* cli: group and clean commands by domain and rename options
* Add `fee` to the Client and CLI APIs
* Normalize core::message and cli commands parameters
* Update `ProjectId`, now alias to `(OrgId, ProjectName)`
* Drop `ProjectDomain`
* Update `Project` to the latest spec containing different fields
* Drop `TransferFromProject` message
* `String32::from_string` now returns `Result<String32, InordinateStringError>`
* `Bytes128::from_vec` now returns `Result<Bytes128, InordinateVectorError>`
* Make `ProjectDomain` a wrapper of `String32` and only support the "rad" domain.
* Add `metadata` field to Project, a vector of at most 128 bytes.
* Drop project fields `description` and `img_url`
* Rename `Client::submit` to `Client::sign_and_submit_call`.
* `Client::create` and `Client::create_with_executor` now require a `host`
  argument. Use `host = url::Host::parse("127.0.0.1").unwrap()` to have the old
  behavior.
* `Client::submit` and  `Client::submit_transaction` now return a wrapped
  future. This allows consumers to distinguish the accepted and applied states
  of a transaction.
* Remove convenience methods for submitting transactions from the client
  - `Client::transfer`
  - `Client::register_project`
  - `Client::create_checkpoint`
  - `Client::set_checkpoint`
  Calls to these functions can be replaced by calls to `Client::submit`.
* Eliminate `ClientWithExecutor`, use `Client::create_with_executor()` instead.
* Eliminate `MemoryClient`. The memory client is now called the emulator and can
  be created with `Client::new_emulator()`.
* All library and binary names have been changed to use dashes instead of
  underscores e.g. `radicle_registry_node` becomes `radicle-registry-node`.
* The error type in `get_dispatch_result` has changed from
  `Option<&'static str>` to `sp_runtime::DispatchError`.
* The `Client` and `ClientT` methods are now `async`.
* The `--block-author` option was replaced with the `--mine` option. A node only
  mines if the option is given.
* Polkadot telemetry was removed

### Addition

* cli: Add `runtime version` command to check the on-chain runtime version
* cli: Add `update-runtime` command to update the on-chain runtime
* cli: Mutually support local account names where only SS58 address were
  supported as params.
* Add `user list` and `user show` CLI commands
* Add `MINIMUM_FEE` to the registry client
* The client emulator now authors blocks when a transaction is submitted.
* Add `TransferFromOrg` message
* Add `Client::get_org` and `Client::list_orgs`
* Add `RegisterOrg` and `UnregisterOrg` messages
* Add Transaction::hash() function
* Offline transaction signing with the following new APIs
  * `ClientT::account_nonce()`
  * `ClientT::genesis_hash()`
  * `Transaction`
  * `Transaction::new_signed()`
* Add block header fetching to the client API
