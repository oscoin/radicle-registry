/// Runtime tests implemented with [MemoryClient].
///
/// High-level runtime tests that only use [MemoryClient] and treat the runtime as a black box.
///
/// The tests in this module concern project registration.
use futures::prelude::*;

use radicle_registry_client::*;

mod common;

#[test]
fn register_project() {
    let client = Client::new_emulator();
    let alice = common::key_pair_from_string("Alice");

    let project_hash = H256::random();
    let checkpoint_id = client
        .create_checkpoint(&alice, project_hash, None)
        .wait()
        .unwrap();
    let params = common::random_register_project_params(checkpoint_id);
    let tx_applied = client.submit(&alice, params.clone()).wait().unwrap();

    let project = client
        .get_project(params.clone().id)
        .wait()
        .unwrap()
        .unwrap();
    assert_eq!(project.id, params.clone().id);
    assert_eq!(project.description, params.description);
    assert_eq!(project.img_url, params.img_url);
    assert_eq!(project.current_cp, checkpoint_id);

    assert_eq!(
        tx_applied.events[0],
        RegistryEvent::ProjectRegistered(params.clone().id, project.account_id).into()
    );

    let has_project = client
        .list_projects()
        .wait()
        .unwrap()
        .iter()
        .any(|id| *id == params.id);
    assert!(has_project, "Registered project not found in project list");

    let checkpoint_ = Checkpoint {
        parent: None,
        hash: project_hash,
    };
    let checkpoint = client
        .get_checkpoint(checkpoint_id)
        .wait()
        .unwrap()
        .unwrap();
    assert_eq!(checkpoint, checkpoint_);
}

#[test]
fn register_project_with_duplicate_id() {
    let client = Client::new_emulator();
    let alice = common::key_pair_from_string("Alice");

    let project_hash = H256::random();
    let checkpoint_id = client
        .create_checkpoint(&alice, project_hash, None)
        .wait()
        .unwrap();

    let params = common::random_register_project_params(checkpoint_id);

    client
        .register_project(&alice, params.clone())
        .wait()
        .unwrap();

    // Duplicate submission with different description and image URL.
    let registration_2 = client
        .submit(
            &alice,
            RegisterProjectParams {
                description: "DESCRIPTION_2".to_string(),
                img_url: "IMG_URL_2".to_string(),
                ..params.clone()
            },
        )
        .wait()
        .unwrap();

    assert_eq!(registration_2.result, Err(None));

    let project = client.get_project(params.id).wait().unwrap().unwrap();

    assert_eq!(params.description, project.description);
    assert_eq!(params.img_url, project.img_url)
}

#[test]
fn register_project_with_bad_checkpoint() {
    let client = Client::new_emulator();
    let alice = common::key_pair_from_string("Alice");

    let checkpoint_id = H256::random();

    let params = common::random_register_project_params(checkpoint_id);

    let tx_applied = client.submit(&alice, params.clone()).wait().unwrap();

    assert_eq!(tx_applied.result, Err(None));

    let no_project = client.get_project(params.id).wait().unwrap();

    assert!(no_project.is_none())
}
