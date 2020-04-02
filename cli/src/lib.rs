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

//! Define the command line parser and interface.

#![allow(clippy::large_enum_variant)]

use radicle_registry_client::*;
use structopt::StructOpt;
use thiserror::Error as ThisError;

pub mod account_storage;

mod command;
use command::{account, org, other, project, user};

/// The type that captures the command line.
#[derive(StructOpt, Clone)]
#[structopt(max_term_width = 80)]
pub struct CommandLine {
    #[structopt(subcommand)]
    pub command: Command,
}

impl CommandLine {
    pub async fn run(self) -> Result<(), CommandError> {
        self.command.run().await
    }
}

/// Network-related command-line options
#[derive(StructOpt, Clone, Debug)]
pub struct NetworkOptions {
    /// IP address or domain name that hosts the RPC API
    #[structopt(
        long,
        default_value = "127.0.0.1",
        env = "RAD_NODE_HOST",
        parse(try_from_str = url::Host::parse),
    )]
    pub node_host: url::Host,
}

impl NetworkOptions {
    pub async fn client(&self) -> Result<Client, Error> {
        Client::create_with_executor(self.node_host.clone()).await
    }
}

/// Transaction-related command-line options
#[derive(StructOpt, Clone)]
pub struct TxOptions {
    /// The name of the local account to be used to sign transactions.
    #[structopt(
        long,
        env = "RAD_TX_AUTHOR",
        value_name = "account_name",
        parse(try_from_str = lookup_account)
    )]
    pub author: ed25519::Pair,

    /// Fee that will be charged to submit transactions.
    /// The higher the fee, the higher the priority of a transaction.
    #[structopt(long, default_value = "1", env = "RAD_TX_FEE", value_name = "fee")]
    pub fee: Balance,
}

fn lookup_account(name: &str) -> Result<ed25519::Pair, String> {
    let accounts = account_storage::list().map_err(|e| format!("{}", e))?;
    match accounts.get(&name.to_string()) {
        Some(account) => Ok(ed25519::Pair::from_seed(&account.seed)),
        None => Err(format!("Could not find local account named '{}'", name)),
    }
}

/// The supported [CommandLine] commands.
/// The commands are grouped by domain.
#[derive(StructOpt, Clone)]
pub enum Command {
    Account(account::Command),
    Org(org::Command),
    Project(project::Command),
    User(user::Command),

    #[structopt(flatten)]
    Other(other::Command),
}

#[async_trait::async_trait]
impl CommandT for Command {
    async fn run(self) -> Result<(), CommandError> {
        match self.clone() {
            Command::Account(cmd) => cmd.run().await,
            Command::Org(cmd) => cmd.run().await,
            Command::Project(cmd) => cmd.run().await,
            Command::User(cmd) => cmd.run().await,
            Command::Other(cmd) => cmd.run().await,
        }
    }
}

/// The trait that every command must implement.
#[async_trait::async_trait]
pub trait CommandT {
    async fn run(self) -> Result<(), CommandError>;
}

/// Error returned by [CommandT::run].
///
/// Implements [From] for client errors and [account_storage] errors.
#[derive(Debug, ThisError)]
pub enum CommandError {
    #[error("Client error: {0}")]
    ClientError(#[from] Error),

    #[error("Transaction {tx_hash} failed in block {block_hash}")]
    FailedTransaction {
        tx_hash: TxHash,
        block_hash: BlockHash,
    },

    #[error("Cannot find org {org_id}")]
    OrgNotFound { org_id: OrgId },

    #[error("Cannot find project {project_name}.{org_id}")]
    ProjectNotFound {
        project_name: ProjectName,
        org_id: OrgId,
    },

    #[error("{0}")]
    AccountStorageError(#[from] account_storage::Error),
}
