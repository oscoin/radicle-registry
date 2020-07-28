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
    storage::{IterableStorageMap, StorageMap, StorageValue as _},
    traits::{Currency, ExistenceRequirement, Randomness as _},
    weights::Pays,
};
use frame_system as system; // required for `decl_module!` to work
use frame_system::{ensure_none, ensure_signed};
use sp_core::crypto::UncheckedFrom;

use radicle_registry_core::*;

use crate::{fees, AccountId, Hash};

mod inherents;

pub use inherents::AuthoringInherentData;

pub trait Trait
where
    // We fix the associated types so that the `Module` code that takes a type of this trait as a
    // parameter does not need to be generic in, say, the `AccountId`, say.
    //
    // Fixing one associated type requires us to also either fix all dependent associated types or
    // restate the associated types bounds.
    //
    // The associated type bounds that depend on the fixed types also need to be restated at the
    // usage site of `Trait`. Currently `Trait` is used for `Store`, `Module`, and
    // `ProvideInherent`. This is due to a limitation with Rusts type checker.
    Self: frame_system::Trait<
        BaseCallFilter = (),
        AccountId = AccountId,
        Origin = crate::Origin,
        Call = crate::Call,
        Hash = Hash,
        OnNewAccount = (),
    >,
    <Self as frame_system::Trait>::Event: From<frame_system::RawEvent<AccountId>>,
    <Self as frame_system::Trait>::OnKilledAccount:
        frame_support::traits::OnKilledAccount<Self::AccountId>,
{
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

/// Funds that are credited to the block author for every block.
pub const BLOCK_REWARD: Balance = rad_to_balance(20);

pub mod store {
    use super::*;

    decl_storage! {
        pub trait Store for Module<T: Trait> as Counter
        where
            // Rust’s type checker is unable to deduce these type bounds from the fact that `T:
            // Trait` altough they are stated in the definition of `Trait`. See the comment in
            // `Trait` for more information.
            <T as frame_system::Trait>::Event: From<frame_system::RawEvent<AccountId>>,
            <T as frame_system::Trait>::OnKilledAccount:
                frame_support::traits::OnKilledAccount<AccountId>,
        {
            // Author of the current block. Is initialized at the beginning of a block with
            // [Call::set_block_author] and not persisted.
            pub BlockAuthor: Option<AccountId>;

            // The below map indexes all retired user and org ids.
            // We use the blake2_128_concat hasher so that the Id
            // can be extracted from the key.
            pub RetiredIds1: map hasher(blake2_128_concat) Id => ();

            // The storage for Orgs, indexed by Id.
            // We use the blake2_128_concat hasher so that the Id
            // can be extracted from the key.
            pub Orgs1: map hasher(blake2_128_concat) Id => Option<state::Orgs1Data>;

            // The storage for Users, indexed by Id.
            // We use the blake2_128_concat hasher so that the Id can be extraced from the key.
            pub Users1: map hasher(blake2_128_concat) Id => Option<state::Users1Data>;

            // We use the blake2_128_concat hasher so that the ProjectId can be extracted from the
            // key.
            pub Projects1: map hasher(blake2_128_concat) ProjectId => Option<state::Projects1Data>;
        }
    }
}

pub use store::Store;

decl_module! {
    pub struct Module<T: Trait> for enum Call where
        origin: T::Origin,
        // Rust’s type checker is unable to deduce these type bounds from the fact that `T:
        // Trait` altough they are stated in the definition of `Trait`. See the comment in
        // `Trait` for more information.
        <T as frame_system::Trait>::Event: From<frame_system::RawEvent<AccountId>>,
        <T as frame_system::Trait>::OnKilledAccount:
            frame_support::traits::OnKilledAccount<AccountId>
    {
        fn deposit_event() = default;
        #[weight = (0, Pays::No)]
        pub fn register_project(origin, message: message::RegisterProject) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let project_id = (message.project_name.clone(), message.project_domain.clone());
            if store::Projects1::get(project_id.clone()).is_some() {
                return Err(RegistryError::DuplicateProjectId.into());
            };

            match &message.project_domain {
                ProjectDomain::Org(org_id) => {
                    let org = store::Orgs1::get(org_id).ok_or(RegistryError::InexistentOrg)?;
                    if !org_has_member_with_account(&org, sender) {
                        return Err(RegistryError::InsufficientSenderPermissions.into());
                    }
                    store::Orgs1::insert(org_id, org.add_project(message.project_name.clone()));
                },
                ProjectDomain::User(user_id) => {
                    let user = store::Users1::get(user_id).ok_or(RegistryError::InexistentUser)?;
                    if user.account_id() != sender {
                        return Err(RegistryError::InsufficientSenderPermissions.into());
                    }
                    store::Users1::insert(user_id, user.add_project(message.project_name.clone()));
                },
            };

            let new_project = state::Projects1Data::new(
                message.metadata
            );
            store::Projects1::insert(project_id, new_project);

            Self::deposit_event(Event::ProjectRegistered(message.project_name, message.project_domain));
            Ok(())
        }

        #[weight = (0, Pays::No)]
        pub fn register_member(origin, message: message::RegisterMember) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let org = store::Orgs1::get(message.org_id.clone()).ok_or(RegistryError::InexistentOrg)?;
            if !org_has_member_with_account(&org, sender) {
                return Err(RegistryError::InsufficientSenderPermissions.into());
            }

            if store::Users1::get(message.user_id.clone()).is_none() {
                return Err(RegistryError::InexistentUser.into());
            }

            if org.members().contains(&message.user_id) {
                return Err(RegistryError::AlreadyAMember.into());
            }

            let org_with_member = org.add_member(message.user_id.clone());
            store::Orgs1::insert(message.org_id.clone(), org_with_member);
            Self::deposit_event(Event::MemberRegistered(message.user_id, message.org_id));
            Ok(())
        }

        #[weight = (0, Pays::No)]
        pub fn register_org(origin, message: message::RegisterOrg) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            ensure_id_is_available(&message.org_id)?;
            let user_id = get_user_id_with_account(sender).ok_or(RegistryError::AuthorHasNoAssociatedUser)?;
            fees::pay_registration_fee(&sender)?;
            let random_account_id = AccountId::unchecked_from(
                pallet_randomness_collective_flip::Module::<T>::random(
                    b"org-account-id",
                )
            );
            let new_org = state::Orgs1Data::new(random_account_id, vec![user_id],  Vec::new());
            store::Orgs1::insert(message.org_id.clone(), new_org);
            store::RetiredIds1::insert(message.org_id.clone(), ());
            Self::deposit_event(Event::OrgRegistered(message.org_id));

            Ok(())
        }

        #[weight = (0, Pays::No)]
        pub fn unregister_org(origin, message: message::UnregisterOrg) -> DispatchResult {
            fn can_be_unregistered(org: state::Orgs1Data, sender: AccountId) -> bool {
                org.projects().is_empty() && get_user_id_with_account(sender)
                    .map(|user_id| org.members() == &[user_id]).unwrap_or(false)
            }

            let sender = ensure_signed(origin)?;

            match store::Orgs1::get(message.org_id.clone()) {
                None => Err(RegistryError::InexistentOrg.into()),
                Some(org) => {
                    if can_be_unregistered(org, sender) {
                        store::Orgs1::remove(message.org_id.clone());
                        Self::deposit_event(Event::OrgUnregistered(message.org_id));
                        Ok(())
                    }
                    else {
                        Err(RegistryError::UnregisterableOrg.into())
                    }
                }
            }
        }

        #[weight = (0, Pays::No)]
        pub fn register_user(origin, message: message::RegisterUser) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            ensure_id_is_available(&message.user_id)?;

            if get_user_with_account(sender).is_some() {
                return Err(RegistryError::UserAccountAssociated.into())
            }

            fees::pay_registration_fee(&sender)?;
            let new_user = state::Users1Data::new(
                sender,
                Vec::new(),
            );
            store::Users1::insert(message.user_id.clone(), new_user);
            store::RetiredIds1::insert(message.user_id.clone(), ());
            Self::deposit_event(Event::UserRegistered(message.user_id));
            Ok(())
        }

        #[weight = (0, Pays::No)]
        pub fn unregister_user(origin, message: message::UnregisterUser) -> DispatchResult {

            let sender = ensure_signed(origin)?;
            let (user_id, user) = get_user_with_account(sender).ok_or(RegistryError::InexistentUser)?;

            if message.user_id != user_id {
                return Err(RegistryError::InsufficientSenderPermissions.into());
            }
            if !user.projects().is_empty() || find_org(|org| org.members().contains(&user_id)).is_some() {
                return Err(RegistryError::UnregisterableUser.into());
            }

            store::Users1::remove(user_id.clone());
            Self::deposit_event(Event::UserUnregistered(user_id));
            Ok(())
        }

        #[weight = (0, Pays::No)]
        pub fn transfer_from_org(origin, message: message::TransferFromOrg) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let org = store::Orgs1::get(message.org_id)
                .ok_or(RegistryError::InexistentOrg)?;

            if org_has_member_with_account(&org, sender) {
                <crate::runtime::Balances as Currency<_>>::transfer(
                    &org.account_id(),
                    &message.recipient,
                    message.amount,
                    ExistenceRequirement::KeepAlive
                )
            }
            else {
                Err(RegistryError::InsufficientSenderPermissions.into())
            }
        }

        #[weight = (0, Pays::No)]
        pub fn transfer(origin, message: message::Transfer) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            <crate::runtime::Balances as Currency<_>>::transfer(
                &sender,
                &message.recipient,
                message.amount,
                ExistenceRequirement::KeepAlive
            )
        }

        #[weight = (0, Pays::No)]
        fn set_block_author(origin, author: AccountId) -> DispatchResult {
            assert!(ensure_none(origin).is_ok(), "set_block_author call is only valid as an inherent");
            assert!(store::BlockAuthor::get().is_none(), "set_block_author can only be called once");
            store::BlockAuthor::put(author);
            Ok(())
        }

        fn on_finalize() {
            let block_author = store::BlockAuthor::take().expect("Block author must be set by an extrinsic");
            let imbalance = crate::runtime::Balances::deposit_creating(&block_author, BLOCK_REWARD);
            drop(imbalance);
        }

    }
}

fn ensure_id_is_available(id: &Id) -> Result<(), RegistryError> {
    if store::Users1::contains_key(id) || store::Orgs1::contains_key(id) {
        Err(RegistryError::IdAlreadyTaken)
    } else if store::RetiredIds1::contains_key(id) {
        Err(RegistryError::IdRetired)
    } else {
        Ok(())
    }
}

fn get_user_id_with_account(account_id: AccountId) -> Option<Id> {
    get_user_with_account(account_id).map(|(id, _)| id)
}

// TODO(xla): This is a naive first version of the check to see if an account is
// already associated to a user. While fine for small dataset this needs to be reworked
// in the future.
pub fn get_user_with_account(account_id: AccountId) -> Option<(Id, state::Users1Data)> {
    store::Users1::iter().find(|(_, user)| user.account_id() == account_id)
}

pub fn find_org(predicate: impl Fn(&state::Orgs1Data) -> bool) -> Option<state::Orgs1Data> {
    store::Orgs1::iter()
        .find(|(_, org)| predicate(org))
        .map(|(_, org)| org)
}

/// Check whether the user associated with the given account_id is a member of the given org.
/// Return false if the account doesn't have an associated user or if said user is not a member
/// of the org.
pub fn org_has_member_with_account(org: &state::Orgs1Data, account_id: AccountId) -> bool {
    match get_user_id_with_account(account_id) {
        Some(user_id) => org.members().contains(&user_id),
        None => false,
    }
}

decl_event!(
    pub enum Event {
        MemberRegistered(Id, Id),
        OrgRegistered(Id),
        OrgUnregistered(Id),
        ProjectRegistered(ProjectName, ProjectDomain),
        UserRegistered(Id),
        UserUnregistered(Id),
    }
);

/// Trait to decode [StorageMap] keys from raw storage keys.
pub trait DecodeKey {
    type Key: parity_scale_codec::Decode;

    /// Decode the given raw storage map `key`. This method is inverse of the private
    /// [`storage_map_final_key`][1] implementation for storage generators. so applying
    /// `decode_key` after `storage_map_final_key` must yield identity as to the original input
    /// key.
    ///
    /// [1]: https://github.com/paritytech/substrate/blob/c50faf2395218e644859611d703d9fe3a4876f5b/frame/support/src/storage/generator/map.rs#L71-L88
    fn decode_key(key: &[u8]) -> Result<Self::Key, parity_scale_codec::Error>;
}

impl DecodeKey for store::Orgs1 {
    type Key = Id;

    fn decode_key(key: &[u8]) -> Result<Id, parity_scale_codec::Error> {
        decode_blake_two128_concat_key(key)
    }
}

impl DecodeKey for store::Projects1 {
    type Key = ProjectId;

    fn decode_key(key: &[u8]) -> Result<ProjectId, parity_scale_codec::Error> {
        decode_blake_two128_concat_key(key)
    }
}

impl DecodeKey for store::Users1 {
    type Key = Id;

    fn decode_key(key: &[u8]) -> Result<Id, parity_scale_codec::Error> {
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
        let org_id = Id::try_from("monadic").unwrap();
        let hashed_key = store::Orgs1::storage_map_final_key(org_id.clone());
        let decoded_key = store::Orgs1::decode_key(&hashed_key).unwrap();
        assert_eq!(decoded_key, org_id);
    }

    /// Test that store::Projects::decode_key after store::Projects::storage_map_final_key
    /// is identical to the original input id.
    #[test]
    fn projects_decode_key_identity() {
        let org_id = Id::try_from("monadic").unwrap();
        let project_name = ProjectName::try_from("radicle".to_string()).unwrap();
        let project_id: ProjectId = (project_name, ProjectDomain::Org(org_id));
        let hashed_key = store::Projects1::storage_map_final_key(project_id.clone());
        let decoded_key = store::Projects1::decode_key(&hashed_key).unwrap();
        assert_eq!(decoded_key, project_id);
    }

    /// Test that store::Users::decode_key after store::Users::storage_map_final_key
    /// is identical the original user id.
    #[test]
    fn users_decode_key_identity() {
        let user_id = Id::try_from("cloudhead").unwrap();
        let hashed_key = store::Users1::storage_map_final_key(user_id.clone());
        let decoded_key = store::Users1::decode_key(&hashed_key).unwrap();
        assert_eq!(decoded_key, user_id);
    }
}
