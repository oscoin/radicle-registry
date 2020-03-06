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

//! Defines [CommandT] trait, structs for all commands and their [CommandT] implementations.
use radicle_registry_client::*;
use structopt::StructOpt;

/// Contextual data for running commands. Created from command line options.
pub struct CommandContext {
    pub author_key_pair: ed25519::Pair,
    pub client: Client,
}

/// Error returned by [CommandT::run].
///
/// Implements [From] for client errors.
#[derive(Debug, derive_more::From)]
pub enum CommandError {
    ClientError(Error),
    FailedTransaction {
        tx_hash: TxHash,
        block_hash: BlockHash,
    },
    ProjectNotFound {
        project_name: ProjectName,
        org_id: OrgId,
    },
}

impl core::fmt::Display for CommandError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            CommandError::ClientError(error) => write!(f, "Client error: {}", error),
            CommandError::FailedTransaction {
                tx_hash,
                block_hash,
            } => write!(f, "Transaction {} failed in block {}", tx_hash, block_hash),
            CommandError::ProjectNotFound {
                project_name,
                org_id,
            } => write!(f, "Cannot find project {}.{}", project_name, org_id),
        }
    }
}

/// Check that a transaction has been applied succesfully.
///
/// If the transaction failed, that is if `tx_applied.result` is `Err`, then we return a
/// [CommandError]. Otherwise we return the `Ok` value of the transaction result.
fn transaction_applied_ok<Message_, T, E>(
    tx_applied: &TransactionApplied<Message_>,
) -> Result<T, CommandError>
where
    Message_: Message<Result = Result<T, E>>,
    T: Copy + Send + 'static,
    E: Send + 'static,
{
    match tx_applied.result {
        Ok(value) => Ok(value),
        Err(_) => Err(CommandError::FailedTransaction {
            tx_hash: tx_applied.tx_hash,
            block_hash: tx_applied.block,
        }),
    }
}

/// Every CLI command must implement this trait.
#[async_trait::async_trait]
pub trait CommandT {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError>;
}

#[derive(StructOpt, Debug, Clone)]
/// Show information for a registered project.
pub struct ShowProject {
    /// The name of the project
    project_name: String32,
    /// The org in which the project is registered.
    org_id: OrgId,
}

#[async_trait::async_trait]
impl CommandT for ShowProject {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let opt_project = command_context
            .client
            .get_project(self.project_name.clone(), self.org_id.clone())
            .await?;

        let project = match opt_project {
            None => {
                return Err(CommandError::ProjectNotFound {
                    project_name: self.project_name.clone(),
                    org_id: self.org_id.clone(),
                });
            }
            Some(project) => project,
        };

        println!("project: {}.{}", project.name, project.org_id);
        println!("checkpoint: {}", project.current_cp);
        Ok(())
    }
}
#[derive(StructOpt, Debug, Clone)]
/// List all projects in the registry
pub struct ListProjects {}

#[async_trait::async_trait]
impl CommandT for ListProjects {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let project_ids = command_context.client.list_projects().await?;
        println!("PROJECTS");
        for (name, org) in project_ids {
            println!("{}.{}", name, org)
        }
        Ok(())
    }
}

#[derive(StructOpt, Debug, Clone)]
/// Register an org.
pub struct RegisterOrg {
    /// Id of the org to register.
    org_id: OrgId,
}

#[async_trait::async_trait]
impl CommandT for RegisterOrg {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let client = &command_context.client;

        let register_org_fut = client
            .sign_and_submit_message(
                &command_context.author_key_pair,
                message::RegisterOrg {
                    org_id: self.org_id.clone(),
                    //TODO(nuno) pass real bid here
                    bid: 10,
                },
            )
            .await?;
        println!("Registering org...");

        let org_registered = register_org_fut.await?;
        transaction_applied_ok(&org_registered)?;
        println!("Org {} is now registered.", self.org_id);
        Ok(())
    }
}

#[derive(StructOpt, Debug, Clone)]
/// Unregister an org.
pub struct UnregisterOrg {
    /// Id of the org to unregister.
    org_id: OrgId,
}

#[async_trait::async_trait]
impl CommandT for UnregisterOrg {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let client = &command_context.client;

        let register_org_fut = client
            .sign_and_submit_message(
                &command_context.author_key_pair,
                message::UnregisterOrg {
                    org_id: self.org_id.clone(),
                    //TODO(nuno): receive real fee
                    bid: 100,
                },
            )
            .await?;
        println!("Unregistering org...");

        let org_unregistered = register_org_fut.await?;
        transaction_applied_ok(&org_unregistered)?;
        println!("Org {} is now unregistered.", self.org_id);
        Ok(())
    }
}

#[derive(StructOpt, Debug, Clone)]
/// Register a project with the given name under the given org.
pub struct RegisterProject {
    /// Name of the project to register.
    project_name: String32,

    /// Org under which to register the project.
    org_id: OrgId,

    /// Project state hash. A hex-encoded 32 byte string. Defaults to all zeros.
    project_hash: Option<H256>,
}

#[async_trait::async_trait]
impl CommandT for RegisterProject {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let client = &command_context.client;

        let create_checkpoint_fut = client
            .sign_and_submit_message(
                &command_context.author_key_pair,
                message::CreateCheckpoint {
                    project_hash: self.project_hash.unwrap_or_default(),
                    previous_checkpoint_id: None,
                    //TODO(nuno): pass real bid here
                    bid: 10,
                },
            )
            .await?;
        println!("creating checkpoint...");

        let checkpoint_created = create_checkpoint_fut.await?;
        let checkpoint_id = transaction_applied_ok(&checkpoint_created)?;
        println!("checkpoint created in block {}", checkpoint_created.block);

        let register_project_fut = client
            .sign_and_submit_message(
                &command_context.author_key_pair,
                message::RegisterProject {
                    project_name: self.project_name.clone(),
                    org_id: self.org_id.clone(),
                    checkpoint_id,
                    metadata: Bytes128::random(),
                    //TODO(nuno): pass real bid here
                    bid: 10,
                },
            )
            .await?;
        println!("registering project...");
        let project_registered = register_project_fut.await?;
        transaction_applied_ok(&project_registered)?;
        println!(
            "project {}.{} registered in block {}",
            self.project_name, self.org_id, project_registered.block,
        );
        Ok(())
    }
}

#[derive(StructOpt, Debug, Clone)]
/// Show the genesis hash the node uses
pub struct ShowGenesisHash {}

#[async_trait::async_trait]
impl CommandT for ShowGenesisHash {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let genesis_hash = command_context.client.genesis_hash();
        println!("Gensis block hash: 0x{}", hex::encode(genesis_hash));
        Ok(())
    }
}

#[derive(StructOpt, Debug, Clone)]
/// Transfer funds to recipient
pub struct Transfer {
    #[structopt(parse(try_from_str = parse_account_id))]
    /// Recipient Account in SS58 address format.
    recipient: AccountId,
    // The amount to transfer.
    funds: Balance,
}

fn parse_account_id(data: &str) -> Result<AccountId, String> {
    sp_core::crypto::Ss58Codec::from_ss58check(data).map_err(|err| format!("{:?}", err))
}

#[async_trait::async_trait]
impl CommandT for Transfer {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let client = &command_context.client;

        let transfer_fut = client
            .sign_and_submit_message(
                &command_context.author_key_pair,
                message::Transfer {
                    recipient: self.recipient,
                    balance: self.funds,
                    //TODO(nuno): pass real bid here
                    bid: 10,
                },
            )
            .await?;
        println!("transferring funds...");
        let transfered = transfer_fut.await?;
        transaction_applied_ok(&transfered)?;
        println!(
            "transferred {} RAD to {} in block {}",
            self.funds, self.recipient, transfered.block,
        );
        Ok(())
    }
}

#[derive(StructOpt, Debug, Clone)]
/// Transfer funds from an org to a recipient.
/// The author needs to be a member of the org.
pub struct TransferOrgFunds {
    /// Id of the org.
    #[structopt(value_name = "org")]
    org_id: OrgId,

    /// Recipient Account in SS58 address format
    #[structopt(parse(try_from_str = parse_account_id))]
    recipient: AccountId,

    // The balance to transfer from the org to the recipient.
    funds: Balance,
}

#[async_trait::async_trait]
impl CommandT for TransferOrgFunds {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let client = &command_context.client;
        let transfer_fut = client
            .sign_and_submit_message(
                &command_context.author_key_pair,
                message::TransferFromOrg {
                    org_id: self.org_id.clone(),
                    recipient: self.recipient,
                    value: self.funds,
                    //TODO(nuno) Pass real bid here
                    bid: 10,
                },
            )
            .await?;
        println!("transferring funds...");
        let transfered = transfer_fut.await?;
        transaction_applied_ok(&transfered)?;
        println!(
            "transferred {} RAD from Org {} to Account {} in block {}",
            self.funds, self.org_id, self.recipient, transfered.block,
        );
        Ok(())
    }
}

#[derive(StructOpt, Debug, Clone)]
/// Show the balance of an account
pub struct ShowBalance {
    #[structopt(
        value_name = "account",
        parse(try_from_str = parse_account_id),
    )]
    /// SS58 address
    account_id: AccountId,
}

#[async_trait::async_trait]
impl CommandT for ShowBalance {
    async fn run(&self, command_context: &CommandContext) -> Result<(), CommandError> {
        let balance = command_context
            .client
            .free_balance(&self.account_id)
            .await?;
        println!("{} RAD", balance);
        Ok(())
    }
}
