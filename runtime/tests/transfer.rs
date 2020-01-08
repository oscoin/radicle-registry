/// Runtime tests implemented with [MemoryClient].
///
/// High-level runtime tests that only use [MemoryClient] and treat the runtime as a black box.
///
/// The tests in this module concern transferring funds.
use futures01::prelude::*;

use radicle_registry_client::*;

mod common;

#[test]
fn transfer_fail() {
    let client = Client::new_emulator();
    let alice = common::key_pair_from_string("Alice");
    let bob = common::key_pair_from_string("Bob").public();

    let balance_alice = client.free_balance(&alice.public()).wait().unwrap();
    let tx_applied = common::submit_ok(
        &client,
        &alice,
        TransferParams {
            recipient: bob,
            balance: balance_alice + 1,
        },
    );
    assert_eq!(tx_applied.result, Err(None));
}

/// Test that we can transfer money to a project and that the project owner can transfer money from
/// a project to another account.
#[test]
fn project_account_transfer() {
    let client = Client::new_emulator();
    let alice = common::key_pair_from_string("Alice");
    let bob = common::key_pair_from_string("Bob").public();
    let project = common::create_project_with_checkpoint(&client, &alice);

    assert_eq!(client.free_balance(&project.account_id).wait().unwrap(), 0);
    common::submit_ok(
        &client,
        &alice,
        TransferParams {
            recipient: project.account_id,
            balance: 2000,
        },
    );
    assert_eq!(
        client.free_balance(&project.account_id).wait().unwrap(),
        2000
    );

    assert_eq!(client.free_balance(&bob).wait().unwrap(), 0);

    common::submit_ok(
        &client,
        &alice,
        TransferFromProjectParams {
            project: project.id.clone(),
            recipient: bob,
            value: 1000,
        },
    );
    assert_eq!(client.free_balance(&bob).wait().unwrap(), 1000);
    assert_eq!(
        client.free_balance(&project.account_id).wait().unwrap(),
        1000
    );
}

#[test]
/// Test that a transfer from a project account fails if the sender is not a project member.
fn project_account_transfer_non_member() {
    let client = Client::new_emulator();
    let alice = common::key_pair_from_string("Alice");
    let bob = common::key_pair_from_string("Bob");
    let project = common::create_project_with_checkpoint(&client, &alice);

    common::submit_ok(
        &client,
        &alice,
        TransferParams {
            recipient: project.account_id,
            balance: 2000,
        },
    );
    assert_eq!(
        client.free_balance(&project.account_id).wait().unwrap(),
        2000
    );

    common::submit_ok(
        &client,
        &bob,
        TransferFromProjectParams {
            project: project.id.clone(),
            recipient: bob.public(),
            value: 1000,
        },
    );

    assert_eq!(
        client.free_balance(&project.account_id).wait().unwrap(),
        2000
    );
}
