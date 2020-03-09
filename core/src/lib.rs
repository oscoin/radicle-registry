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

//! Basic types used in the Radicle Registry.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(alloc_prelude)]

extern crate alloc;

use alloc::vec::Vec;

use sp_core::{ed25519, H256};
use sp_runtime::traits::BlakeTwo256;

pub use sp_runtime::DispatchError;

pub mod message;
pub mod state;

pub mod bytes128;
pub use bytes128::Bytes128;

pub mod string32;
pub use string32::String32;

mod org_id;
pub use org_id::OrgId;

mod error;
pub use error::RegistryError;

/// The hashing algorightm to use
pub type Hashing = BlakeTwo256;

/// Identifier for accounts, an Ed25519 public key.
///
/// Each account has an associated [message::AccountBalance] and [message::Index].
pub type AccountId = ed25519::Public;

/// The non-negative balance of anything storing the amount of currency.
/// It can be used to represent the value of anything describing an amount,
/// e.g. an account balance, the value of a fee, etc.
pub type Balance = u128;

/// The id of a project. Used as storage key.
pub type ProjectId = (ProjectName, OrgId);

/// The name a project is registered with.
pub type ProjectName = String32;

/// Org
///
/// Different from [state::Org] in which this type gathers
/// both the [`OrgId`] and the other [`Org`] fields, respectively stored
/// as an Org's storage key and data, into one complete type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Org {
    // Unique id of an Org.
    pub id: OrgId,

    /// See [state::Org::account_id]
    pub account_id: AccountId,

    /// See [state::Org::members]
    pub members: Vec<AccountId>,

    /// See [state::Org::projects]
    pub projects: Vec<ProjectName>,
}

impl Org {
    pub fn new(id: OrgId, org: state::Org) -> Org {
        Org {
            id,
            account_id: org.account_id,
            members: org.members,
            projects: org.projects,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// The name of the project, unique within its org.
    pub name: ProjectName,

    /// The Org to which the project belongs to.
    pub org_id: OrgId,

    /// See [state::Project::current_cp]
    pub current_cp: CheckpointId,

    /// See [state::Project::metadata]
    pub metadata: Bytes128,
}

impl Project {
    /// Build a [crate::Project] given all its properties obtained from storage.
    pub fn new(name: ProjectName, org_id: OrgId, project: state::Project) -> Self {
        Project {
            name,
            org_id,
            current_cp: project.current_cp,
            metadata: project.metadata,
        }
    }
}

pub type CheckpointId = H256;
