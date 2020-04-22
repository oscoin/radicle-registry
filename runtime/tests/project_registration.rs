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

/// Runtime tests implemented with [MemoryClient].
///
/// High-level runtime tests that only use [MemoryClient] and treat the runtime as a black box.
///
/// The tests in this module concern project registration.
use radicle_registry_client::*;
use radicle_registry_test_utils::*;

#[async_std::test]
async fn register_project() {
    let client = Client::new_emulator();
    let alice = key_pair_from_string("Alice");

    let project_hash = H256::random();
    let msg = message::CreateCheckpoint {
        project_hash,
        previous_checkpoint_id: None,
    };
    submit_ok(&client, &alice, msg.clone())
        .await
        .result
        .unwrap();
    let checkpoint_id = Client::checkpoint_id(msg.previous_checkpoint_id, msg.project_hash);

    let register_org = random_register_org_message();
    submit_ok(&client, &alice, register_org.clone()).await;

    // The org needs funds to submit transactions.
    let org = client
        .get_org(register_org.org_id.clone())
        .await
        .unwrap()
        .unwrap();
    let initial_balance = 1000;
    transfer(&client, &alice, org.account_id, initial_balance).await;

    let random_fee = random_balance();
    let message = random_register_project_message(register_org.org_id.clone(), checkpoint_id);
    let tx_applied = submit_ok_with_fee(&client, &alice, message.clone(), random_fee).await;

    let project = client
        .get_project(message.clone().project_name, message.clone().org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(project.name.clone(), message.project_name.clone());
    assert_eq!(project.org_id.clone(), message.org_id.clone());
    assert_eq!(project.current_cp.clone(), checkpoint_id);
    assert_eq!(project.metadata.clone(), message.metadata.clone());

    assert_eq!(
        tx_applied.events[0],
        RegistryEvent::ProjectRegistered(message.clone().project_name, message.clone().org_id)
            .into()
    );

    let has_project = client
        .list_projects()
        .await
        .unwrap()
        .iter()
        .any(|id| *id == (message.project_name.clone(), message.org_id.clone()));
    assert!(has_project, "Registered project not found in project list");

    let checkpoint_ = state::Checkpoint {
        parent: None,
        hash: project_hash,
    };
    let checkpoint = client.get_checkpoint(checkpoint_id).await.unwrap().unwrap();
    assert_eq!(checkpoint, checkpoint_);

    let org: Org = client
        .get_org(register_org.org_id.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(org.projects.len(), 1);
    assert!(
        org.projects.contains(&project.name.clone()),
        "Org does not contain the added project."
    );

    assert_eq!(
        client.free_balance(&org.account_id).await.unwrap(),
        initial_balance - random_fee,
        "The tx fee was not charged properly."
    );
}

#[async_std::test]
async fn register_project_with_inexistent_org() {
    let client = Client::new_emulator();
    let alice = key_pair_from_string("Alice");

    let project_hash = H256::random();
    let msg = message::CreateCheckpoint {
        project_hash,
        previous_checkpoint_id: None,
    };
    submit_ok(&client, &alice, msg.clone())
        .await
        .result
        .unwrap();
    let checkpoint_id = Client::checkpoint_id(msg.previous_checkpoint_id, msg.project_hash);

    let inexistent_org_id = random_org_id();
    let message = random_register_project_message(inexistent_org_id, checkpoint_id);
    let tx_applied = submit_ok(&client, &alice, message.clone()).await;

    assert_eq!(tx_applied.result, Err(RegistryError::InexistentOrg.into()));
}

#[async_std::test]
async fn register_project_with_duplicate_id() {
    let client = Client::new_emulator();
    let alice = key_pair_from_string("Alice");

    let msg = message::CreateCheckpoint {
        project_hash: H256::random(),
        previous_checkpoint_id: None,
    };
    submit_ok(&client, &alice, msg.clone())
        .await
        .result
        .unwrap();
    let checkpoint_id = Client::checkpoint_id(msg.previous_checkpoint_id, msg.project_hash);

    let org_id = random_org_id();
    let register_org = message::RegisterOrg {
        org_id: org_id.clone(),
    };
    submit_ok(&client, &alice, register_org.clone()).await;

    // The org needs funds to submit transactions.
    let org = client.get_org(org_id.clone()).await.unwrap().unwrap();
    transfer(&client, &alice, org.account_id, 1000).await;

    let message = random_register_project_message(org_id.clone(), checkpoint_id);
    submit_ok(&client, &alice, message.clone()).await;

    // Duplicate submission with a different metadata.
    let registration_2 = submit_ok(
        &client,
        &alice,
        message::RegisterProject {
            metadata: Bytes128::random(),
            ..message.clone()
        },
    )
    .await;

    assert_eq!(
        registration_2.result,
        Err(RegistryError::DuplicateProjectId.into())
    );

    let project = client
        .get_project(message.project_name, message.org_id)
        .await
        .unwrap()
        .unwrap();
    // Assert that the project data was not altered during the
    // attempt to re-register the already existing project.
    assert_eq!(message.metadata, project.metadata);

    let org = client.get_org(org_id).await.unwrap().unwrap();
    // Assert that the number of projects in the involved Org didn't change.
    assert_eq!(org.projects.len(), 1);
    assert!(
        org.projects.contains(&project.name),
        "Registered project not found in the org project list",
    );
}

#[async_std::test]
async fn register_project_with_bad_checkpoint() {
    let client = Client::new_emulator();
    let alice = key_pair_from_string("Alice");

    let checkpoint_id = H256::random();

    let org_id = random_org_id();
    let register_project = random_register_project_message(org_id.clone(), checkpoint_id);
    let register_org = message::RegisterOrg {
        org_id: org_id.clone(),
    };
    submit_ok(&client, &alice, register_org.clone()).await;

    // The org needs funds to submit transactions.
    let org = client.get_org(org_id.clone()).await.unwrap().unwrap();
    transfer(&client, &alice, org.account_id, 1000).await;

    let tx_applied = submit_ok(&client, &alice, register_project.clone()).await;

    assert_eq!(
        tx_applied.result,
        Err(RegistryError::InexistentCheckpointId.into())
    );

    assert!(client
        .get_project(register_project.project_name, register_project.org_id)
        .await
        .unwrap()
        .is_none());
}

#[async_std::test]
async fn register_project_with_bad_actor() {
    let client = Client::new_emulator();
    let god_actor = key_pair_from_string("Alice");
    let bad_actor = key_pair_from_string("BadActor");
    // The bad actor needs funds to submit transactions.
    transfer(&client, &god_actor, bad_actor.public(), 1000).await;

    // The good actor creates an org, of which becomes its single member.
    let org_id = random_org_id();
    let register_org = message::RegisterOrg {
        org_id: org_id.clone(),
    };
    submit_ok(&client, &god_actor, register_org.clone()).await;

    // The bad actor attempts to register a project within that org.
    let initial_balance = client.free_balance(&bad_actor.public()).await.unwrap();
    let register_project = random_register_project_message(org_id.clone(), H256::random());
    let random_fee = random_balance();
    let tx_applied =
        submit_ok_with_fee(&client, &bad_actor, register_project.clone(), random_fee).await;

    assert_eq!(
        tx_applied.result,
        Err(RegistryError::InsufficientSenderPermissions.into())
    );

    // Check that the bad actor payed for the transaction anyway.
    assert_eq!(
        client.free_balance(&bad_actor.public()).await.unwrap(),
        initial_balance - random_fee,
        "The tx fee was not charged properly."
    );

    assert!(client
        .get_project(register_project.project_name, register_project.org_id)
        .await
        .unwrap()
        .is_none());
}
