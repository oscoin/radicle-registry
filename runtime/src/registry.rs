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

use alloc::vec;
use alloc::vec::Vec;

use frame_support::{
    decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    storage::StorageMap as _,
    storage::StoragePrefixedMap,
    traits::{Currency, ExistenceRequirement, Randomness as _},
    weights::SimpleDispatchInfo,
};
use frame_system as system; // required for `decl_module!` to work
use frame_system::ensure_signed;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::traits::Hash as _;

use radicle_registry_core::*;

use crate::{AccountId, Hash, Hashing};

pub trait Trait:
    frame_system::Trait<AccountId = AccountId, Origin = crate::Origin, Hash = Hash>
{
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

pub mod store {
    use super::*;

    decl_storage! {
        pub trait Store for Module<T: Trait> as Registry {
            // The storage for Orgs, indexed by OrgId.
            // We use the blake2_128_concat hasher so that the OrgId
            // can be extracted from the key.
            pub Orgs: map hasher(blake2_128_concat) OrgId => Option<state::Org>;

            // The storage for Users, indexed by UserId.
            // We use the blake2_128_concat hasher so that the UserId can be extraced from the key.
            pub Users: map hasher(blake2_128_concat) UserId => Option<state::User>;

            // We use the blake2_128_concat hasher so that the ProjectId can be extracted from the
            // key.
            pub Projects: map hasher(blake2_128_concat) ProjectId => Option<state::Project>;
            // The below map indexes each existing project's id to the
            // checkpoint id that it was registered with.
            pub InitialCheckpoints: map hasher(blake2_256) ProjectId => Option<CheckpointId>;
            // The below map indexes each checkpoint's id to the checkpoint
            // it points to, should it exist.
            pub Checkpoints: map hasher(blake2_256) CheckpointId => Option<state::Checkpoint>;
        }
    }
}

pub use store::Store;

/// Returns true iff `checkpoint_id` descends from `initial_cp_id`.
fn descends_from_initial_checkpoint(
    checkpoint_id: CheckpointId,
    initial_cp_id: CheckpointId,
) -> bool {
    if checkpoint_id == initial_cp_id {
        return true;
    };

    let mut ancestor_id = checkpoint_id;

    // The number of storage requests made in this loop grows linearly
    // with the size of the checkpoint's ancestry.
    //
    // The loop's total runtime will also depend on the performance of
    // each `store::StorageMap::get` request.
    while let Some(cp) = store::Checkpoints::get(ancestor_id) {
        match cp.parent {
            None => return false,
            Some(cp_id) => {
                if cp_id == initial_cp_id {
                    return true;
                } else {
                    ancestor_id = cp_id;
                }
            }
        }
    }

    false
}

// TODO Note on `DispatchError`
//
// This datatype is now an `enum`, and some of its variants can be used to
// provide richer errors in the case of runtime failures.
//
// This will be handled in a future issue.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn register_project(origin, message: message::RegisterProject) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let org = match store::Orgs::get(message.org_id.clone()) {
                None => return Err(RegistryError::InexistentOrg.into()),
                Some(o) => o,
            };

            if !org.members.contains(&sender) {
                return Err(RegistryError::InsufficientSenderPermissions.into());
            }

            if store::Checkpoints::get(message.checkpoint_id).is_none() {
                return Err(RegistryError::InexistentCheckpointId.into())
            }

            let project_id = (message.project_name.clone(), message.org_id.clone());

            if store::Projects::get(project_id.clone()).is_some() {
                return Err(RegistryError::DuplicateProjectId.into());
            };

            let new_project = state::Project {
                current_cp: message.checkpoint_id,
                metadata: message.metadata
            };

            store::Projects::insert(project_id.clone(), new_project);
            store::Orgs::insert(message.org_id.clone(), org.add_project(message.project_name.clone()));
            store::InitialCheckpoints::insert(project_id, message.checkpoint_id);

            Self::deposit_event(Event::ProjectRegistered(message.project_name, message.org_id));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn register_org(origin, message: message::RegisterOrg) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            match store::Orgs::get(message.org_id.clone()) {
                None => {},
                Some(_) => return Err(RegistryError::DuplicateOrgId.into()),
            }

            let random_account_id = AccountId::unchecked_from(
                pallet_randomness_collective_flip::Module::<T>::random(
                    b"org-account-id",
                )
            );

            let new_org = state::Org {
                account_id: random_account_id,
                members: vec![sender],
                projects: Vec::new(),
            };
            store::Orgs::insert(message.org_id.clone(), new_org);
            Self::deposit_event(Event::OrgRegistered(message.org_id));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn unregister_org(origin, message: message::UnregisterOrg) -> DispatchResult {
            fn can_be_unregistered(org: state::Org, sender: AccountId) -> bool {
                org.members == vec![sender] && org.projects.is_empty()
            }

            let sender = ensure_signed(origin)?;

            match store::Orgs::get(message.org_id.clone()) {
                None => Err(RegistryError::InexistentOrg.into()),
                Some(org) => {
                    if can_be_unregistered(org, sender) {
                        store::Orgs::remove(message.org_id.clone());
                        Self::deposit_event(Event::OrgUnregistered(message.org_id));
                        Ok(())
                    }
                    else {
                        Err(RegistryError::UnregisterableOrg.into())
                    }
                }
            }
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn register_user(origin, message: message::RegisterUser) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            if store::Users::get(message.user_id.clone()).is_some() {
                return Err(RegistryError::DuplicateUserId.into())
            }

            // TODO(xla): This is a naive first version of the check to see if an account is
            // already associated to a user. While fine for small dataset this needs to be reworked
            // in the future.
            for user in store::Users::iter() {
                if sender == user.account_id {
                    return Err(RegistryError::UserAccountAssociated.into())
                }
            }

            let new_user = state::User {
                account_id: sender,
                projects: Vec::new(),
            };
            store::Users::insert(message.user_id.clone(), new_user);
            Self::deposit_event(Event::UserRegistered(message.user_id));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn unregister_user(origin, message: message::UnregisterUser) -> DispatchResult {
            fn can_be_unregistered(user: state::User, sender: AccountId) -> bool {
                user.account_id == sender && user.projects.is_empty()
            }

            let sender = ensure_signed(origin)?;

            match store::Users::get(message.user_id.clone()) {
                None => Err(RegistryError::InexistentUser.into()),
                Some(user) => {
                    if can_be_unregistered(user, sender) {
                        store::Users::remove(message.user_id.clone());
                        Self::deposit_event(Event::UserUnregistered(message.user_id));
                        Ok(())
                    }
                    else {
                        Err(RegistryError::NonUnregisterableUser.into())
                    }
                }
            }
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn transfer_from_org(origin, message: message::TransferFromOrg) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let org = match store::Orgs::get(message.org_id) {
                None => return Err(RegistryError::InexistentOrg.into()),
                Some(o) => o,
            };
            if org.members.contains(&sender) {
                <crate::Balances as Currency<_>>::transfer(
                    &org.account_id,
                    &message.recipient,
                    message.value, ExistenceRequirement::KeepAlive
                )
            }
            else {
                Err(RegistryError::InsufficientSenderPermissions.into())
            }
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn create_checkpoint(
            origin,
            message: message::CreateCheckpoint,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            match message.previous_checkpoint_id {
                None => {}
                Some(cp_id) => {
                    match store::Checkpoints::get(cp_id) {
                        None => return Err(RegistryError::InexistentCheckpointId.into()),
                        Some(_) => {}
                    }
                }
            };

            let checkpoint = state::Checkpoint {
                parent: message.previous_checkpoint_id,
                hash: message.project_hash,
            };
            let checkpoint_id = Hashing::hash_of(&checkpoint);
            store::Checkpoints::insert(checkpoint_id, checkpoint);

            Self::deposit_event(Event::CheckpointCreated(checkpoint_id));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn set_checkpoint(
            origin,
            message: message::SetCheckpoint,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            if store::Checkpoints::get(message.new_checkpoint_id).is_none() {
                return Err(RegistryError::InexistentCheckpointId.into())
            }
            let project_id = (message.project_name.clone(), message.org_id.clone());
            let opt_project = store::Projects::get(project_id.clone());
            let opt_org = store::Orgs::get(message.org_id.clone());
            let new_project = match (opt_project, opt_org) {
                (Some(prj), Some(org)) => {
                    if !org.members.contains(&sender) {
                        return Err(RegistryError::InsufficientSenderPermissions.into())
                    }
                    state::Project {
                        current_cp: message.new_checkpoint_id,
                        ..prj
                    }
                }
                _ => return Err(RegistryError::InexistentProjectId.into()),

            };

            let initial_cp = match store::InitialCheckpoints::get(project_id.clone()) {
                None => return Err(RegistryError::InexistentInitialProjectCheckpoint.into()),
                Some(cp) => cp,
            };
            if !descends_from_initial_checkpoint(message.new_checkpoint_id, initial_cp) {
                return Err(RegistryError::InvalidCheckpointAncestry.into())
            }

            store::Projects::insert(project_id, new_project);

            Self::deposit_event(Event::CheckpointSet(
                message.project_name.clone(),
                message.org_id.clone(),
                message.new_checkpoint_id
            ));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::InsecureFreeNormal]
        pub fn transfer(origin, message: message::Transfer) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            <crate::Balances as Currency<_>>::transfer(
                &sender,
                &message.recipient,
                message.balance,
                ExistenceRequirement::KeepAlive
            )
        }
    }
}
decl_event!(
    pub enum Event {
        CheckpointCreated(CheckpointId),
        CheckpointSet(ProjectName, OrgId, CheckpointId),
        OrgRegistered(OrgId),
        OrgUnregistered(OrgId),
        ProjectRegistered(ProjectName, OrgId),
        UserRegistered(UserId),
        UserUnregistered(UserId),
    }
);

/// DecodeKey trait
///
/// A DecodeKey type must implement the decode_key function and
/// it is free to choose what decoding algorithm it prefers.
///
/// DecodedKey::Key is the expected decoded key type
///
pub trait DecodeKey {
    type Key: parity_scale_codec::Decode;

    /// Decode the given raw storage map `key`. It's the inverse of
    /// [frame_support::generator::StorageMap::storage_map_final_key],
    /// so applying `decode_key` after `storage_map_final_key` must
    /// yield identity as to the original input key.
    fn decode_key(key: &[u8]) -> Result<Self::Key, parity_scale_codec::Error>;
}

impl DecodeKey for store::Orgs {
    type Key = OrgId;

    fn decode_key(key: &[u8]) -> Result<OrgId, parity_scale_codec::Error> {
        decode_blake_two128_concat_key(key)
    }
}

impl DecodeKey for store::Projects {
    type Key = ProjectId;

    fn decode_key(key: &[u8]) -> Result<ProjectId, parity_scale_codec::Error> {
        decode_blake_two128_concat_key(key)
    }
}

impl DecodeKey for store::Users {
    type Key = UserId;

    fn decode_key(key: &[u8]) -> Result<UserId, parity_scale_codec::Error> {
        decode_blake_two128_concat_key(key)
    }
}

/// Decode a blake_two128_concat hashed key to the inferred type K.
///
/// The key consists of the concatenation of the module prefix hash (16 bytes),
/// the storage prefix hash (16 bytes), the key hash (16 bytes), and
/// finally the raw key. See the actual implementation of this key concatenation at
/// [frame_support::storage::generator::StorageMap::storage_map_final_key].
pub fn decode_blake_two128_concat_key<K: parity_scale_codec::Decode>(
    key: &[u8],
) -> Result<K, parity_scale_codec::Error> {
    let final_storage_key_prefix_length = 48;
    let mut id_bytes = &key[final_storage_key_prefix_length..];
    K::decode(&mut id_bytes)
}

#[cfg(test)]
mod test {
    use core::convert::TryFrom;
    use frame_support::storage::generator::StorageMap;

    use super::*;

    /// Test that store::Orgs::decode_key after store::Orgs::storage_map_final_key
    /// is identical to the original input id.
    #[test]
    fn orgs_decode_key_identity() {
        let org_id = OrgId::try_from("monadic").unwrap();
        let hashed_key = store::Orgs::storage_map_final_key(org_id.clone());
        let decoded_key = store::Orgs::decode_key(&hashed_key).unwrap();
        assert_eq!(decoded_key, org_id);
    }

    /// Test that store::Projects::decode_key after store::Projects::storage_map_final_key
    /// is identical to the original input id.
    #[test]
    fn projects_decode_key_identity() {
        let org_id = OrgId::try_from("monadic").unwrap();
        let project_name = ProjectName::try_from("radicle".to_string()).unwrap();
        let project_id: ProjectId = (project_name, org_id);
        let hashed_key = store::Projects::storage_map_final_key(project_id.clone());
        let decoded_key = store::Projects::decode_key(&hashed_key).unwrap();
        assert_eq!(decoded_key, project_id);
    }

    /// Test that store::Users::decode_key after store::Users::storage_map_final_key
    /// is identical the original user id.
    #[test]
    fn users_decode_key_identity() {
        let user_id = UserId::try_from("cloudhead").unwrap();
        let hashed_key = store::Users::storage_map_final_key(user_id.clone());
        let decoded_key = store::Users::decode_key(&hashed_key).unwrap();
        assert_eq!(decoded_key, user_id);
    }
}
