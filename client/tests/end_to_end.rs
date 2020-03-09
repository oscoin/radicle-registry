//! Test the client against a running node.
//!
//! Note that chain state is shared between the test runs.

use serial_test::serial;

use radicle_registry_client::*;
use radicle_registry_runtime::fees::{BaseFee, Fee};
use radicle_registry_test_utils::*;

#[async_std::test]
#[serial]
async fn register_project() {
    let _ = env_logger::try_init();
    let node_host = url::Host::parse("127.0.0.1").unwrap();
    let client = Client::create_with_executor(node_host).await.unwrap();
    let alice = key_pair_from_string("Alice");
    let initial_balance_alice = client.free_balance(&alice.public()).await.unwrap();

    let create_checkpoint_bid = random_balance();
    let project_hash = H256::random();
    let checkpoint_id = submit_ok(
        &client,
        &alice,
        message::CreateCheckpoint {
            project_hash,
            previous_checkpoint_id: None,
            bid: create_checkpoint_bid,
        },
    )
    .await
    .result
    .unwrap();

    let after_balance_alice = client.free_balance(&alice.public()).await.unwrap();
    assert_eq!(
        after_balance_alice,
        initial_balance_alice - create_checkpoint_bid,
        "The tx author should have paid for all tx fees."
    );

    let initial_balance_alice = client.free_balance(&alice.public()).await.unwrap();
    let register_org_bid = random_balance();
    let org_id = random_string32();
    let register_org_message = message::RegisterOrg {
        org_id: org_id.clone(),
        bid: register_org_bid,
    };
    let org_registered_tx = submit_ok(&client, &alice, register_org_message.clone()).await;
    assert_eq!(org_registered_tx.result, Ok(()));
    let after_balance_alice = client.free_balance(&alice.public()).await.unwrap();
    assert_eq!(
        after_balance_alice,
        initial_balance_alice - create_checkpoint_bid,
        "The tx author should have paid for all tx fees."
    );

    let org = client.get_org(org_id.clone()).await.unwrap().unwrap();
    // The org needs some balance to run transactions.
    grant_funds(&client, &alice, org.account_id.clone(), 1000).await;
    let initial_balance_alice = client.free_balance(&alice.public()).await.unwrap();
    let initial_balance_org = client.free_balance(&org.account_id).await.unwrap();

    let register_project_message = random_register_project_message(org_id.clone(), checkpoint_id);
    let project_name = register_project_message.project_name.clone();
    let org_id = register_project_message.org_id.clone();
    let tx_applied = submit_ok(&client, &alice, register_project_message.clone()).await;
    assert_eq!(tx_applied.result, Ok(()));

    let project = client
        .get_project(project_name.clone(), org_id.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(project.name.clone(), project_name.clone());
    assert_eq!(project.org_id.clone(), org_id.clone());
    assert_eq!(
        project.current_cp.clone(),
        register_project_message.checkpoint_id
    );
    assert_eq!(project.metadata.clone(), register_project_message.metadata);

    assert_eq!(
        tx_applied.events[0],
        RegistryEvent::ProjectRegistered(project_name.clone(), org_id.clone()).into()
    );

    let checkpoint = client.get_checkpoint(checkpoint_id).await.unwrap().unwrap();
    let checkpoint_ = state::Checkpoint {
        parent: None,
        hash: project_hash,
    };
    assert_eq!(checkpoint, checkpoint_);

    assert!(
        client
            .get_project(project_name.clone(), org_id.clone())
            .await
            .unwrap()
            .is_some(),
        "Registered project not found in project list"
    );

    let org: Org = client.get_org(org_id.clone()).await.unwrap().unwrap();
    assert!(
        org.projects.contains(&project_name),
        format!(
            "Expected project id {} in Org {} with projects {:?}",
            project_name, org_id, org.projects
        )
    );

    assert_eq!(
        client.free_balance(&alice.public()).await.unwrap(),
        initial_balance_alice - BaseFee.value(),
        "The tx author should have (only) paid for the base fee."
    );
    assert_eq!(
        client.free_balance(&org.account_id).await.unwrap(),
        initial_balance_org - (register_project_message.bid - BaseFee.value()),
        "The org should have (only) paid for the tip."
    );
}

#[async_std::test]
#[serial]
async fn register_org() {
    let _ = env_logger::try_init();
    let node_host = url::Host::parse("127.0.0.1").unwrap();
    let client = Client::create_with_executor(node_host).await.unwrap();
    let alice = key_pair_from_string("Alice");
    let initial_balance_alice = client.free_balance(&alice.public()).await.unwrap();

    let register_org_message = random_register_org_message();
    let tx_applied = submit_ok(&client, &alice, register_org_message.clone()).await;

    assert_eq!(
        tx_applied.events[0],
        RegistryEvent::OrgRegistered(register_org_message.org_id.clone()).into()
    );
    assert_eq!(tx_applied.result, Ok(()));

    let opt_org = client
        .get_org(register_org_message.org_id.clone())
        .await
        .unwrap();
    assert!(opt_org.is_some(), "Registered org not found in orgs list");
    let org = opt_org.unwrap();
    assert_eq!(org.id, register_org_message.org_id);
    assert_eq!(org.members, vec![alice.public()]);
    assert!(org.projects.is_empty());

    assert_eq!(
        client.free_balance(&alice.public()).await.unwrap(),
        initial_balance_alice - register_org_message.bid,
        "The tx author should have paid for all tx fees."
    );
}

#[async_std::test]
/// Submit a transaction with an invalid genesis hash and expect an error.
async fn invalid_transaction() {
    let _ = env_logger::try_init();
    let node_host = url::Host::parse("127.0.0.1").unwrap();
    let client = Client::create_with_executor(node_host).await.unwrap();
    let alice = key_pair_from_string("Alice");
    let initial_balance_alice = client.free_balance(&alice.public()).await.unwrap();

    let bid = random_balance();
    let transfer_tx = Transaction::new_signed(
        &alice,
        message::Transfer {
            recipient: alice.public(),
            balance: 1000,
            bid,
        },
        TransactionExtra {
            nonce: 0,
            genesis_hash: Hash::zero(),
        },
    );

    let response = client.submit_transaction(transfer_tx).await;

    assert_eq!(
        client.free_balance(&alice.public()).await.unwrap(),
        initial_balance_alice - bid,
        "The tx author should have paid for all tx fees."
    );
    match response {
        Err(Error::Other(_)) => (),
        Err(error) => panic!("Unexpected error {:?}", error),
        Ok(_) => panic!("Transaction was accepted unexpectedly"),
    }
}
