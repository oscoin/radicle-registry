/// Runtime tests implemented with [MemoryClient].
///
/// High-level runtime tests that only use [MemoryClient] and treat the runtime as a black box.
///
/// The tests in this module concern checkpoint creation and setting project
/// checkpoints.
use futures01::prelude::*;

use radicle_registry_client::*;

mod common;

#[test]
fn set_checkpoint() {
    let client = Client::new_emulator();
    let charles = common::key_pair_from_string("Charles");

    let project = common::create_project_with_checkpoint(&client, &charles);

    let project_hash2 = H256::random();
    let new_checkpoint_id = common::submit_ok(
        &client,
        &charles,
        CreateCheckpointParams {
            project_hash: project_hash2,
            previous_checkpoint_id: Some(project.current_cp),
        },
    )
    .result
    .unwrap();

    common::submit_ok(
        &client,
        &charles,
        SetCheckpointParams {
            project_id: project.id.clone(),
            new_checkpoint_id,
        },
    );

    let new_project = client.get_project(project.id).wait().unwrap().unwrap();
    assert_eq!(new_checkpoint_id, new_project.current_cp)
}

#[test]
fn set_checkpoint_without_permission() {
    let client = Client::new_emulator();
    let eve = common::key_pair_from_string("Eve");

    let project = common::create_project_with_checkpoint(&client, &eve);

    let project_hash2 = H256::random();
    let new_checkpoint_id = common::submit_ok(
        &client,
        &eve,
        CreateCheckpointParams {
            project_hash: project_hash2,
            previous_checkpoint_id: Some(project.current_cp),
        },
    )
    .result
    .unwrap();

    let frank = common::key_pair_from_string("Frank");
    let tx_applied = common::submit_ok(
        &client,
        &frank,
        SetCheckpointParams {
            project_id: project.id.clone(),
            new_checkpoint_id,
        },
    );

    let updated_project = client
        .get_project(project.id.clone())
        .wait()
        .unwrap()
        .unwrap();
    assert_eq!(tx_applied.result, Err(DispatchError::Other("")));
    assert_eq!(updated_project.current_cp, project.current_cp);
    assert_ne!(updated_project.current_cp, new_checkpoint_id);
}

#[test]
fn fail_to_set_nonexistent_checkpoint() {
    let client = Client::new_emulator();
    let david = common::key_pair_from_string("David");

    let project = common::create_project_with_checkpoint(&client, &david);

    let garbage = CheckpointId::random();

    let tx_applied = common::submit_ok(
        &client,
        &david,
        SetCheckpointParams {
            project_id: project.id.clone(),
            new_checkpoint_id: garbage,
        },
    );

    assert_eq!(tx_applied.result, Err(DispatchError::Other("")));
    let updated_project = client
        .get_project(project.id.clone())
        .wait()
        .unwrap()
        .unwrap();
    assert_eq!(updated_project.current_cp, project.current_cp);
    assert_ne!(updated_project.current_cp, garbage);
}

#[test]
fn set_fork_checkpoint() {
    let client = Client::new_emulator();
    let grace = common::key_pair_from_string("Grace");

    let project = common::create_project_with_checkpoint(&client, &grace);

    let mut current_cp = project.current_cp;

    // How many checkpoints to create.
    let n = 5;
    let mut checkpoints: Vec<CheckpointId> = Vec::with_capacity(n);
    for _ in 0..n {
        let new_checkpoint_id = common::submit_ok(
            &client,
            &grace,
            CreateCheckpointParams {
                project_hash: H256::random(),
                previous_checkpoint_id: (Some(current_cp)),
            },
        )
        .result
        .unwrap();
        current_cp = new_checkpoint_id;
        checkpoints.push(new_checkpoint_id);
    }

    let forked_checkpoint_id = common::submit_ok(
        &client,
        &grace,
        CreateCheckpointParams {
            project_hash: H256::random(),
            previous_checkpoint_id: (Some(checkpoints[2])),
        },
    )
    .result
    .unwrap();

    common::submit_ok(
        &client,
        &grace,
        SetCheckpointParams {
            project_id: project.id.clone(),
            new_checkpoint_id: forked_checkpoint_id,
        },
    );

    let project_1 = client.get_project(project.id).wait().unwrap().unwrap();

    assert_eq!(project_1.current_cp, forked_checkpoint_id)
}
