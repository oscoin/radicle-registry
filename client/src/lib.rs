// Radicle Registry
// Copyright (C) 2019 Monadic GmbH <radicle@monadic.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Clients for the radicle registry.
//!
//! This crate provides a high-level registry ledger [Client] and all related types.
//!
//! Create a remote node client with [Client::create].
//!
//! [Client::new_emulator] creates a client that emulates the ledger in memory without having a
//! local node.
//!
//! [Client::create_with_executor] creates a client that uses its own runtime to spawn futures.
//!
//! # Transactions
//!
//! A [Transaction] can be created and signed offline using [Transaction::new_signed]. This
//! constructor requires the account nonce and genesis hash of the chain. Those can be obtained
//! using [ClientT::account_nonce] and [ClientT::genesis_hash]. See
//! `./client/examples/transaction_signing.rs`.
//!
use std::sync::Arc;

use parity_scale_codec::{Decode, FullCodec};

use frame_support::storage::generator::{StorageMap, StorageValue};
use radicle_registry_runtime::{balances, registry, Runtime};

mod backend;
mod error;
mod interface;
mod message;
mod transaction;

pub use crate::interface::*;

/// Client to interact with the radicle registry ledger via an implementation of [ClientT].
///
/// The client can either use a full node as the backend (see [Client::create]) or emulate the
/// registry in memory with [Client::new_emulator].
#[derive(Clone)]
pub struct Client {
    backend: Arc<dyn backend::Backend + Sync + Send>,
}

impl Client {
    /// Connects to a registry node running on the given host and returns a [Client].
    ///
    /// Fails if it cannot connect to a node. Uses websocket over port 9944.
    pub async fn create(host: url::Host) -> Result<Self, Error> {
        let backend = backend::RemoteNode::create(host).await?;
        Ok(Self::new(backend))
    }

    /// Same as [Client::create] but calls to the client spawn futures in an executor owned by the
    /// client.
    ///
    /// This makes it possible to call [Future::wait] on the client even if that function is called
    /// in an event loop of another executor.
    pub async fn create_with_executor(host: url::Host) -> Result<Self, Error> {
        let backend = backend::RemoteNodeWithExecutor::create(host).await?;
        Ok(Self::new(backend))
    }

    /// Create a new client that emulates the registry ledger in memory. See
    /// [backend::emulator::Emulator] for details.
    pub fn new_emulator() -> Self {
        Self::new(backend::Emulator::new())
    }

    fn new(backend: impl backend::Backend + Sync + Send + 'static) -> Self {
        Client {
            backend: Arc::new(backend),
        }
    }

    /// Fetch a value from the state storage based on a [StorageValue] implementation provided by
    /// the runtime.
    ///
    /// ```ignore
    /// client.fetch_value::<frame_balance::TotalIssuance<Runtime>, _>();
    /// ```
    async fn fetch_value<S: StorageValue<Value>, Value: FullCodec + Send + 'static>(
        &self,
    ) -> Result<S::Query, Error>
    where
        S::Query: Send + 'static,
    {
        let backend = self.backend.clone();
        let maybe_data = backend
            .fetch(S::storage_value_final_key().as_ref(), None)
            .await?;
        let value = match maybe_data {
            Some(data) => {
                let value = Decode::decode(&mut &data[..])?;
                Some(value)
            }
            None => None,
        };
        Ok(S::from_optional_value_to_query(value))
    }

    /// Fetch a value from a map in the state storage based on a [StorageMap] implementation
    /// provided by the runtime.
    ///
    /// ```ignore
    /// client.fetch_map_value::<frame_system::AccountNonce<Runtime>, _, _>(account_id);
    /// ```
    async fn fetch_map_value<
        S: StorageMap<Key, Value>,
        Key: FullCodec,
        Value: FullCodec + Send + 'static,
    >(
        &self,
        key: Key,
    ) -> Result<S::Query, Error>
    where
        S::Query: Send + 'static,
    {
        let backend = self.backend.clone();
        // We cannot move this code into the async block. The compiler complains about a processing
        // cycle (E0391)
        let key = S::storage_map_final_key(key);
        let maybe_data = backend.fetch(&key, None).await?;
        let value = match maybe_data {
            Some(data) => {
                let value = Decode::decode(&mut &data[..])?;
                Some(value)
            }
            None => None,
        };
        Ok(S::from_optional_value_to_query(value))
    }
}

#[async_trait::async_trait]
impl ClientT for Client {
    async fn submit_transaction<Call_: Message>(
        &self,
        transaction: Transaction<Call_>,
    ) -> Result<Response<TransactionApplied<Call_>, Error>, Error> {
        let backend = self.backend.clone();
        let tx_applied_future = backend.submit(transaction.extrinsic).await?;
        Ok(Box::pin(async move {
            let tx_applied = tx_applied_future.await?;
            let events = tx_applied.events;
            let tx_hash = tx_applied.tx_hash;
            let block = tx_applied.block;
            let result = Call_::result_from_events(events.clone())?;
            Ok(TransactionApplied {
                tx_hash,
                block,
                events,
                result,
            })
        }))
    }

    async fn sign_and_submit_call<Call_: Message>(
        &self,
        author: &ed25519::Pair,
        call: Call_,
    ) -> Result<Response<TransactionApplied<Call_>, Error>, Error> {
        let account_id = author.public();
        let key_pair = author.clone();
        let genesis_hash = self.genesis_hash();
        let client = self.clone();
        let nonce = client.account_nonce(&account_id).await?;
        let transaction = Transaction::new_signed(
            &key_pair,
            call,
            TransactionExtra {
                nonce,
                genesis_hash,
            },
        );
        let tx_applied_fut = client.submit_transaction(transaction).await?;
        Ok(Box::pin(async move {
            let tx_applied = tx_applied_fut.await?;
            let events = tx_applied.events;
            let tx_hash = tx_applied.tx_hash;
            let block = tx_applied.block;
            let result = Call_::result_from_events(events.clone())?;
            Ok(TransactionApplied {
                tx_hash,
                block,
                events,
                result,
            })
        }))
    }

    fn genesis_hash(&self) -> Hash {
        self.backend.get_genesis_hash()
    }

    async fn account_nonce(&self, account_id: &AccountId) -> Result<Index, Error> {
        self.fetch_map_value::<frame_system::AccountNonce<Runtime>, _, _>(*account_id)
            .await
    }

    async fn free_balance(&self, account_id: &AccountId) -> Result<Balance, Error> {
        self.fetch_map_value::<balances::FreeBalance<Runtime>, _, _>(account_id.clone())
            .await
    }

    async fn get_project(&self, id: ProjectId) -> Result<Option<Project>, Error> {
        self.fetch_map_value::<registry::store::Projects, _, _>(id)
            .await
    }

    async fn list_projects(&self) -> Result<Vec<ProjectId>, Error> {
        self.fetch_value::<registry::store::ProjectIds, _>().await
    }

    async fn get_checkpoint(&self, id: CheckpointId) -> Result<Option<Checkpoint>, Error> {
        self.fetch_map_value::<registry::store::Checkpoints, _, _>(id)
            .await
    }
}

/*/// Turn a 0.3 future into a boxed 0.1 future trait object.
fn future03_compat<'a, Ok, Error>(
    f: impl Future03<Output = Result<Ok, Error>> + 'a + Send,
) -> Box<dyn Future<Item = Ok, Error = Error> + Send + 'a> {
    Box::new(futures::compat::Compat::new(Box::pin(f)))
}*/

#[cfg(test)]
mod test {
    use super::*;

    /// Assert that [Client] implements [Sync], [Send] and has a `'static` lifetime bound.
    ///
    /// The code does not need to run, we only want it to compile.
    #[allow(dead_code)]
    fn client_is_sync_send_static() {
        fn is_sync_send(_x: impl Sync + Send + 'static) {}
        is_sync_send(Client::new_emulator());
    }
}
