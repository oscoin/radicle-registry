use alloc::format;
use alloc::prelude::v1::*;
use alloc::vec;
use codec::{Decode, Encode, Error as CodecError, Input};
use sr_primitives::weights::SimpleDispatchInfo;
use srml_support::{
    decl_event, decl_module, decl_storage, dispatch::Result as DispatchResult,
    storage::StorageMap as _, storage::StorageValue as _,
};

use sr_std::str::FromStr;

use substrate_primitives::H256;

use srml_system as system;
use srml_system::ensure_signed;

use crate::AccountId;

/// Type to represent project names and domains.
///
/// Since their lengths are limited to 32 characters, a smart constructor is
/// provided to check validity.
#[derive(Encode, Clone, Debug, Eq, PartialEq)]
pub struct String32(String);

impl String32 {
    pub fn from_string(s: String) -> Result<Self, String> {
        if s.len() > 32 {
            Err(format!(
                "The provided string's length exceeded 32 characters: {:?}",
                s
            ))
        } else {
            Ok(String32(s))
        }
    }
}

impl FromStr for String32 {
    type Err = String;

    /// This function only raises an error if the `String` it is passed is
    /// longer than 32 characters.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        String32::from_string(s.to_string())
    }
}

impl Decode for String32 {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let decoded: String = String::decode(input)?;
        if decoded.len() > 32 {
            Err(From::from("String32 length was more than 32 characters."))
        } else {
            Ok(String32(decoded))
        }
    }
}

#[test]
fn encode_then_decode() {
    let string = String32::from_string(String::from("ôítÏйгますいщαφδвы")).unwrap();

    let encoded = string.encode();

    let decoded = <String32>::decode(&mut &encoded[..]).unwrap();

    assert_eq!(string, decoded)
}

/// The name a project is registered with.
pub type ProjectName = String32;

/// The domain under which the project's name is registered.
///
/// At present, the domain must be `rad`, alhtough others may be allowed in
/// the future.
pub type ProjectDomain = String32;

pub type ProjectId = (ProjectName, ProjectDomain);

pub type CheckpointId = H256;

/// A project's version. Used in checkpointing.
pub type Version = String;

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: ProjectId,
    pub description: String,
    pub img_url: String,
    pub members: Vec<AccountId>,
    pub current_cp: CheckpointId,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
pub struct RegisterProjectParams {
    pub id: ProjectId,
    pub description: String,
    pub img_url: String,
    pub checkpoint_id: CheckpointId,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub parent: Option<CheckpointId>,
    pub hash: H256,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
pub struct CreateCheckpointParams {
    pub checkpoint_id: CheckpointId,
    pub project_hash: H256,
    pub previous_checkpoint: Option<CheckpointId>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
pub struct SetCheckpointParams {
    pub project_id: ProjectId,
    pub new_checkpoint_id: CheckpointId,
}

pub trait Trait: srml_system::Trait<AccountId = AccountId, Origin = crate::Origin> {
    type Event: From<Event> + Into<<Self as srml_system::Trait>::Event>;
}

pub mod store {
    use super::*;

    decl_storage! {
        pub trait Store for Module<T: Trait> as Counter {
            pub Projects: map ProjectId => Option<Project>;
            pub InitialCheckpoints: map ProjectId => Option<CheckpointId>;
            pub ProjectIds: Vec<ProjectId>;
            pub Checkpoints: map CheckpointId => Option<Checkpoint>;
        }
    }
}

pub use store::Store;

/// Given a checkpoint, return its oldest ancestor.
fn get_root_checkpoint(checkpoint_id: CheckpointId) -> CheckpointId {
    // At the end of this loop, the value of `ancestor_id` will be
    // the ID of the first ancestor of the checkpoint in
    // `params: SetCheckpointParams`.
    //
    // The number of storage requests made in this loop grows linearly
    // with the size of the checkpoint's ancestry.
    //
    // The loop's total runtime will also depend on the performance of
    // each `store::StorageMap::get` request.
    let mut ancestor_id = checkpoint_id;
    while let Some(cp) = store::Checkpoints::get(ancestor_id) {
        match cp.parent {
            None => break,
            Some(cp_id) => ancestor_id = cp_id,
        }
    }

    ancestor_id
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = SimpleDispatchInfo::FreeNormal]
        pub fn register_project(origin, params: RegisterProjectParams) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let project_id = params.id.clone();
            let project = Project {
                id: project_id.clone(),
                description: params.description,
                img_url: params.img_url,
                members: vec![sender],
                current_cp: params.checkpoint_id
            };

            store::Projects::insert(project_id.clone(), project);
            store::ProjectIds::append_or_put(vec![project_id.clone()]);
            store::InitialCheckpoints::insert(project_id.clone(), params.checkpoint_id);

            Self::deposit_event(Event::ProjectRegistered(project_id.clone()));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FreeNormal]
        pub fn create_checkpoint(
            origin,
            params: CreateCheckpointParams,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let checkpoint = Checkpoint {
                parent: params.previous_checkpoint,
                hash: params.project_hash,
            };
            store::Checkpoints::insert(params.checkpoint_id, checkpoint);

            Self::deposit_event(Event::CheckpointCreated(params.checkpoint_id));
            Ok(())
        }

        #[weight = SimpleDispatchInfo::FreeNormal]
        pub fn set_checkpoint(
            origin,
            params: SetCheckpointParams,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let opt_project = store::Projects::get(params.project_id.clone());
            let new_project = match opt_project {
                None => return Err("The provided project ID is not associated with any project."),
                Some(prj) => {
                    if !prj.members.contains(&sender) {
                        return Err("The `set_checkpoint` transaction sender is not a member of the project.")
                    }
                    Project {
                        current_cp: params.new_checkpoint_id,
                        ..prj
                    }
                }
            };

            let ancestor_id = get_root_checkpoint(params.new_checkpoint_id);
            if Some(ancestor_id) != store::InitialCheckpoints::get(params.project_id.clone()) {
                return Err("The provided checkpoint ID is not a descendant of the project's first checkpoint.")
            }

            store::Projects::insert(new_project.id.clone(), new_project.clone());

            Self::deposit_event(Event::CheckpointSet(new_project.id, params.new_checkpoint_id));
            Ok(())
        }
    }
}
decl_event!(
    pub enum Event {
        ProjectRegistered(ProjectId),
        CheckpointCreated(CheckpointId),
        CheckpointSet(ProjectId, CheckpointId),
    }
);
