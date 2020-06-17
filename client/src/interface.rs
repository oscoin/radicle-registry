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

//! Provide an abstract trait for the registry client and the necessary types.
//!
//! The [ClientT] trait defines one method for each transaction of the registry ledger as well as
//! methods to get the ledger state.
use futures::future::BoxFuture;

pub use radicle_registry_core::*;

pub use radicle_registry_runtime::{BlockNumber, Hash, Header, RuntimeVersion};

pub use radicle_registry_runtime::{registry::Event as RegistryEvent, Balance, Event};
pub use sp_core::crypto::{
    Pair as CryptoPair, Public as CryptoPublic, SecretStringError as CryptoError,
};
pub use sp_core::{ed25519, H256};

pub use crate::error::Error;
pub use crate::message::Message;
pub use crate::transaction::{Transaction, TransactionExtra};

/// The hash of a block. Uniquely identifies a block.
#[doc(inline)]
pub type BlockHash = Hash;

/// The hash of a transaction. Uniquely identifies a transaction.
#[doc(inline)]
pub type TxHash = Hash;

/// The header of a block
#[doc(inline)]
pub type BlockHeader = Header;

/// Org
///
/// Different from [state::OrgV1] in which this type gathers
/// both the [`Id`] and the other [`Org`] fields, respectively stored
/// as an Org's storage key and data, into one complete type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Org {
    // Unique id of an Org.
    pub id: Id,

    /// See [state::OrgV1::account_id]
    pub account_id: AccountId,

    /// See [state::OrgV1::members]
    pub members: Vec<Id>,

    /// See [state::OrgV1::projects]
    pub projects: Vec<ProjectName>,
}

impl Org {
    pub fn new(id: Id, org: state::Orgs1Data) -> Org {
        match org {
            state::Orgs1Data::V1(org) => Org {
                id,
                account_id: org.account_id,
                members: org.members,
                projects: org.projects,
            },
        }
    }
}

/// User
///
/// Different from [state::UserV1] in which this type gathers
/// both the [`Id`] and the other [`User`] fields, respectively stored
/// as an User's storage key and data, into one complete type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct User {
    // Unique id of a User.
    pub id: Id,

    /// See [state::UserV1::account_id]
    pub account_id: AccountId,

    /// See [state::UserV1::projects]
    pub projects: Vec<ProjectName>,
}

impl User {
    pub fn new(id: Id, user: state::Users1Data) -> Self {
        match user {
            state::Users1Data::V1(user) => Self {
                id,
                account_id: user.account_id,
                projects: user.projects,
            },
        }
    }
}

/// Project
///
/// Different from [state::ProjectV1] in which this type gathers
/// [`ProjectName`], [`ProjectRegistrant`] and the other [`Project`] fields into one complete type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// The name of the project, unique within its org.
    pub name: ProjectName,

    /// The registrant to whom the project belongs.
    pub registrant: ProjectRegistrant,

    /// See [state::ProjectV1::current_cp]
    pub current_cp: CheckpointId,

    /// See [state::ProjectV1::metadata]
    pub metadata: Bytes128,
}

impl Project {
    /// Build a [crate::Project] given all its properties obtained from storage.
    pub fn new(name: ProjectName, registrant: ProjectRegistrant, project: state::Projects1Data) -> Self {
        match project {
            state::Projects1Data::V1(project) => Project {
                name,
                registrant,
                current_cp: project.current_cp,
                metadata: project.metadata,
            },
        }
    }
}

/// Checkpoint
///
/// Different from [state::CheckpointV1] in which this type gathers
/// both the [`CheckpointId`] and the other [`Checkpoint`] fields, respectively stored
/// as an Checkpoint's storage key and data, into one complete type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    /// Checkpoint ID.
    pub id: CheckpointId,
    /// Previous checkpoint in the project history.
    pub parent: Option<CheckpointId>,
    /// Hash that identifies a project’s off-chain data.
    pub hash: H256,
}

impl Checkpoint {
    pub fn new(cp: state::Checkpoints1Data) -> Self {
        match cp {
            state::Checkpoints1Data::V1(cp) => Self {
                id: cp.id(),
                parent: cp.parent,
                hash: cp.hash,
            },
        }
    }
}

/// Result of a transaction being included in a block.
///
/// Returned after submitting an transaction to the blockchain.
#[derive(Clone, Debug)]
pub struct TransactionIncluded<Message_: Message> {
    pub tx_hash: TxHash,
    /// The hash of the block the transaction is included in.
    pub block: Hash,
    /// Events emitted by this transaction
    pub events: Vec<Event>,
    /// The result of the runtime message.
    ///
    /// See [Message::result_from_events].
    pub result: Message_::Result,
}

/// Return type for all [ClientT] methods.
pub type Response<T, Error> = BoxFuture<'static, Result<T, Error>>;

/// Trait for ledger clients sending transactions and looking up state.
#[async_trait::async_trait]
pub trait ClientT {
    /// Submit a signed transaction.
    ///
    /// ```no_run
    /// # use radicle_registry_client::*;
    /// # async fn example<M: Message>(client: Client, tx: Transaction<M>) -> Result<(), Error> {
    ///
    /// // Submit the transaction to the node.
    /// //
    /// // If this is successful the transaction has been accepted by the node. The node will then
    /// // dissemniate the transaction to the network.
    /// //
    /// // This call fails if the transaction is invalid or if the RPC communication with the node
    /// // failed.
    /// let tx_included_fut = client.submit_transaction(tx).await?;
    ///
    /// // We can now wait for the transaction to be included in a block.
    /// //
    /// // This will error if the transaction becomes invalid (for example due to the nonce) or if
    /// // we fail to retrieve the transaction state from the node.
    /// //
    /// // This will not error if the transaction errored while applying. See
    /// // TransactionIncluded::result for that.
    /// let tx_included = tx_included_fut.await?;
    ///
    /// Ok(())
    /// # }
    /// ```
    ///
    /// See the `getting_started` example for more details.
    async fn submit_transaction<Message_: Message>(
        &self,
        transaction: Transaction<Message_>,
    ) -> Result<Response<TransactionIncluded<Message_>, Error>, Error>;

    /// Sign and submit a ledger message as a transaction to the blockchain.
    ///
    /// Same as [ClientT::submit_transaction] but takes care of signing the message.
    async fn sign_and_submit_message<Message_: Message>(
        &self,
        author: &ed25519::Pair,
        message: Message_,
        fee: Balance,
    ) -> Result<Response<TransactionIncluded<Message_>, Error>, Error>;

    /// Fetch the nonce for the given account from the chain state
    async fn account_nonce(
        &self,
        account_id: &AccountId,
    ) -> Result<state::AccountTransactionIndex, Error>;

    /// Fetch the header of the given block hash
    async fn block_header(&self, block_hash: BlockHash) -> Result<BlockHeader, Error>;

    /// Fetch the header of the best chain tip
    async fn block_header_best_chain(&self) -> Result<BlockHeader, Error>;

    /// Return the gensis hash of the chain we are communicating with.
    fn genesis_hash(&self) -> Hash;

    async fn free_balance(&self, account_id: &AccountId) -> Result<Balance, Error>;

    async fn get_org(&self, org_id: Id) -> Result<Option<Org>, Error>;

    async fn list_orgs(&self) -> Result<Vec<Id>, Error>;

    async fn get_user(&self, user_id: Id) -> Result<Option<User>, Error>;

    async fn list_users(&self) -> Result<Vec<Id>, Error>;

    async fn get_project(
        &self,
        project_name: ProjectName,
        project_registrant: ProjectRegistrant,
    ) -> Result<Option<Project>, Error>;

    async fn list_projects(&self) -> Result<Vec<ProjectId>, Error>;

    async fn get_checkpoint(&self, id: CheckpointId) -> Result<Option<Checkpoint>, Error>;

    async fn onchain_runtime_version(&self) -> Result<RuntimeVersion, Error>;
}
