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

//! Define the commands supported by the CLI related to Orgs.

use super::*;

/// Org related commands
#[derive(StructOpt, Clone)]
pub enum Command {
    List(List),
    Show(Show),
    Transfer(Transfer),
    Register(Register),
    Unregister(Unregister),
}

#[async_trait::async_trait]
impl CommandT for Command {
    async fn run(&self) -> Result<(), CommandError> {
        match self {
            Command::Show(cmd) => cmd.run().await,
            Command::List(cmd) => cmd.run().await,
            Command::Register(cmd) => cmd.run().await,
            Command::Unregister(cmd) => cmd.run().await,
            Command::Transfer(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Clone)]
/// List all orgs in the registry
pub struct List {
    #[structopt(flatten)]
    network_options: NetworkOptions,
}

#[async_trait::async_trait]
impl CommandT for List {
    async fn run(&self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;
        let org_ids = client.list_orgs().await?;
        println!("ORGS ({})", org_ids.len());
        for org_id in org_ids {
            println!("{}", org_id)
        }
        Ok(())
    }
}

#[derive(StructOpt, Clone)]
/// Show information for a registered org.
pub struct Show {
    /// The id of the org
    org_id: OrgId,

    #[structopt(flatten)]
    network_options: NetworkOptions,
}

#[async_trait::async_trait]
impl CommandT for Show {
    async fn run(&self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;
        let org = client
            .get_org(self.org_id.clone())
            .await?
            .ok_or(CommandError::OrgNotFound {
                org_id: self.org_id.clone(),
            })?;

        println!("id: {}", org.id);
        println!("account_id: {}", org.account_id);
        println!("members: {:?}", org.members);
        println!("projects: {:?}", org.projects);
        Ok(())
    }
}

#[derive(StructOpt, Clone)]
/// Register an org.
pub struct Register {
    /// Id of the org to register.
    org_id: OrgId,

    #[structopt(flatten)]
    network_options: NetworkOptions,

    #[structopt(flatten)]
    tx_options: TxOptions,
}

#[async_trait::async_trait]
impl CommandT for Register {
    async fn run(&self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;

        let register_org_fut = client
            .sign_and_submit_message(
                &self.tx_options.author,
                message::RegisterOrg {
                    org_id: self.org_id.clone(),
                },
                self.tx_options.fee,
            )
            .await?;
        println!("Registering org...");

        let org_registered = register_org_fut.await?;
        transaction_applied_ok(&org_registered)?;
        println!("Org {} is now registered.", self.org_id);
        Ok(())
    }
}

#[derive(StructOpt, Clone)]
/// Unregister an org.
pub struct Unregister {
    /// Id of the org to unregister.
    org_id: OrgId,

    #[structopt(flatten)]
    network_options: NetworkOptions,

    #[structopt(flatten)]
    tx_options: TxOptions,
}

#[async_trait::async_trait]
impl CommandT for Unregister {
    async fn run(&self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;

        let register_org_fut = client
            .sign_and_submit_message(
                &self.tx_options.author,
                message::UnregisterOrg {
                    org_id: self.org_id.clone(),
                },
                self.tx_options.fee,
            )
            .await?;
        println!("Unregistering org...");

        let org_unregistered = register_org_fut.await?;
        transaction_applied_ok(&org_unregistered)?;
        println!("Org {} is now unregistered.", self.org_id);
        Ok(())
    }
}

#[derive(StructOpt, Clone)]
/// Transfer funds from an org to a recipient.
/// The author needs to be a member of the org.
pub struct Transfer {
    /// Id of the org.
    #[structopt(value_name = "org")]
    org_id: OrgId,

    /// Recipient Account in SS58 address format
    #[structopt(parse(try_from_str = parse_account_id))]
    recipient: AccountId,

    // The balance to transfer from the org to the recipient.
    funds: Balance,

    #[structopt(flatten)]
    network_options: NetworkOptions,

    #[structopt(flatten)]
    tx_options: TxOptions,
}

#[async_trait::async_trait]
impl CommandT for Transfer {
    async fn run(&self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;
        let transfer_fut = client
            .sign_and_submit_message(
                &self.tx_options.author,
                message::TransferFromOrg {
                    org_id: self.org_id.clone(),
                    recipient: self.recipient,
                    value: self.funds,
                },
                self.tx_options.fee,
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
