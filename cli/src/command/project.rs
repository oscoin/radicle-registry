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

//! Define the commands supported by the CLI related to Projects.

use super::*;
use structopt::clap::arg_enum;

/// Project related commands
#[derive(StructOpt, Clone)]
pub enum Command {
    /// List all projects in the registry
    List(List),
    /// Register a project with the given name under the given org.
    Register(Register),
    /// Show information for a registered project.
    Show(Show),
}

#[async_trait::async_trait]
impl CommandT for Command {
    async fn run(self) -> Result<(), CommandError> {
        match self {
            Command::List(cmd) => cmd.run().await,
            Command::Register(cmd) => cmd.run().await,
            Command::Show(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Clone)]
pub struct Show {
    /// The name of the project
    project_name: ProjectName,

    /// The org in which the project is registered.
    org_id: Id,

    #[structopt(flatten)]
    network_options: NetworkOptions,
}

#[async_trait::async_trait]
impl CommandT for Show {
    async fn run(self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;

        let project_domain = ProjectDomain::Org(self.org_id.clone());
        let project = client
            .get_project(self.project_name.clone(), project_domain.clone())
            .await?
            .ok_or(CommandError::ProjectNotFound {
                project_name: self.project_name.clone(),
                project_domain,
            })?;
        println!("Project: {}.{:?}", project.name, project.domain);
        println!("Checkpoint: {}", project.current_cp);
        Ok(())
    }
}

#[derive(StructOpt, Clone)]
pub struct List {
    #[structopt(flatten)]
    network_options: NetworkOptions,
}

#[async_trait::async_trait]
impl CommandT for List {
    async fn run(self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;
        let project_ids = client.list_projects().await?;
        println!("PROJECTS ({})", project_ids.len());
        for (name, org) in project_ids {
            println!("{}.{:?}", name, org)
        }
        Ok(())
    }
}

#[derive(StructOpt, Clone)]
pub struct Register {
    /// Name of the project to register.
    project_name: ProjectName,

    /// The type of domain under which to register the project.
    #[structopt(
        possible_values = &DomainType::variants(),
        case_insensitive = true,
    )]
    domain_type: DomainType,

    /// The id of the domain under which to register the project.
    domain_id: Id,

    /// Project state hash. A hex-encoded 32 byte string. Defaults to all zeros.
    project_hash: Option<H256>,

    #[structopt(flatten)]
    network_options: NetworkOptions,

    #[structopt(flatten)]
    tx_options: TxOptions,
}

#[async_trait::async_trait]
impl CommandT for Register {
    async fn run(self) -> Result<(), CommandError> {
        let client = self.network_options.client().await?;
        let create_checkpoint_fut = client
            .sign_and_submit_message(
                &self.tx_options.author,
                message::CreateCheckpoint {
                    project_hash: self.project_hash.unwrap_or_default(),
                    previous_checkpoint_id: None,
                },
                self.tx_options.fee,
            )
            .await?;
        announce_tx("Creating checkpoint...");

        let checkpoint_created = create_checkpoint_fut.await?;
        let checkpoint_id = checkpoint_created.result?;
        println!("✓ Checkpoint created in block {}", checkpoint_created.block);

        let project_domain = match self.domain_type {
            DomainType::Org => ProjectDomain::Org(self.domain_id),
            DomainType::User => ProjectDomain::User(self.domain_id),
        };
        let register_project_fut = client
            .sign_and_submit_message(
                &self.tx_options.author,
                message::RegisterProject {
                    project_name: self.project_name.clone(),
                    project_domain: project_domain.clone(),
                    checkpoint_id,
                    metadata: Bytes128::random(),
                },
                self.tx_options.fee,
            )
            .await?;
        announce_tx("Registering project...");

        let project_registered = register_project_fut.await?;
        project_registered.result?;
        println!(
            "✓ Project {}.{:?} registered in block {}",
            self.project_name, project_domain, project_registered.block,
        );
        Ok(())
    }
}

arg_enum! {
    #[derive(Clone, Eq, PartialEq, Debug)]
    enum DomainType {
        Org,
        User,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_project_domain_from_org() {
        for org_input in &["org", "oRg", "ORG"] {
            let res = DomainType::from_str(org_input);
            assert_eq!(res, Ok(DomainType::Org));
        }
    }

    #[test]
    fn test_project_domain_from_user() {
        for user_input in &["user", "usEr", "USER"] {
            let res = DomainType::from_str(user_input);
            assert_eq!(res, Ok(DomainType::User));
        }
    }
}
