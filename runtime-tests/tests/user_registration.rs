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
/// The tests in this module concern user registration.
use radicle_registry_client::*;
use radicle_registry_test_utils::*;

#[async_std::test]
async fn register_user() {
    let (client, _) = Client::new_emulator();
    let alice = key_pair_from_string("Alice");
    let initial_balance = client.free_balance(&alice.public()).await.unwrap();

    let register_user_message = random_register_user_message();
    let random_fee = random_balance();
    let tx_included =
        submit_ok_with_fee(&client, &alice, register_user_message.clone(), random_fee).await;

    assert!(tx_included
        .events
        .contains(&RegistryEvent::UserRegistered(register_user_message.user_id.clone()).into()));

    assert!(
        user_exists(&client, register_user_message.user_id.clone()).await,
        "User not found in users list",
    );

    assert_eq!(
        client.free_balance(&alice.public()).await.unwrap(),
        initial_balance - random_fee,
        "The tx fee was not charged properly."
    );

    let user: User = client
        .get_user(register_user_message.user_id.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.id, register_user_message.user_id);
    assert!(user.projects.is_empty());
}

#[async_std::test]
async fn register_user_with_duplicate_id() {
    let (client, _) = Client::new_emulator();
    let alice = key_pair_from_string("Alice");
    let register_user_message = random_register_user_message();

    let tx_included_once = submit_ok(&client, &alice, register_user_message.clone()).await;
    assert!(tx_included_once.result.is_ok());

    let tx_included_twice = submit_ok(&client, &alice, register_user_message.clone()).await;
    assert_eq!(
        tx_included_twice.result,
        Err(RegistryError::DuplicateUserId.into())
    )
}

#[async_std::test]
async fn register_user_with_already_associated_account() {
    let (client, _) = Client::new_emulator();
    let alice = key_pair_from_string("Alice");
    let register_first_user_message = random_register_user_message();

    let tx_included_first = submit_ok(&client, &alice, register_first_user_message.clone()).await;
    assert!(tx_included_first.result.is_ok());

    // Register a different user with the same account.
    let register_second_user_message = random_register_user_message();
    let tx_included_twice = submit_ok(&client, &alice, register_second_user_message.clone()).await;
    assert_eq!(
        tx_included_twice.result,
        Err(RegistryError::UserAccountAssociated.into())
    )
}

#[async_std::test]
async fn unregister_user() {
    let (client, _) = Client::new_emulator();
    let alice = key_pair_from_string("Alice");
    let register_user_message = random_register_user_message();

    // Registration.
    let tx_included = submit_ok(&client, &alice, register_user_message.clone()).await;
    assert!(tx_included
        .events
        .contains(&RegistryEvent::UserRegistered(register_user_message.user_id.clone()).into()));
    assert!(tx_included.result.is_ok());
    assert!(
        user_exists(&client, register_user_message.user_id.clone()).await,
        "User not found in users list"
    );

    // Unregistration.
    let initial_balance = client.free_balance(&alice.public()).await.unwrap();

    let unregister_user_message = message::UnregisterUser {
        user_id: register_user_message.user_id.clone(),
    };
    let random_fee = random_balance();
    let tx_unregister_applied =
        submit_ok_with_fee(&client, &alice, unregister_user_message.clone(), random_fee).await;
    assert!(tx_unregister_applied.result.is_ok());
    assert!(
        !user_exists(&client, register_user_message.user_id.clone()).await,
        "The user was not expected to exist"
    );
    assert_eq!(
        client.free_balance(&alice.public()).await.unwrap(),
        initial_balance - random_fee,
        "The tx fee was not charged properly."
    );
}

#[async_std::test]
async fn unregister_user_member_of_an_org() {
    let (client, _) = Client::new_emulator();
    let (author, user_id) = key_pair_with_associated_user(&client).await;

    // Have user registering an org, which sets the associated user as its single member.
    let register_org = random_register_org_message();
    submit_ok(&client, &author, register_org.clone()).await;
    let org = client.get_org(register_org.org_id).await.unwrap().unwrap();
    assert_eq!(org.members, vec![user_id.clone()]);

    // Unregistration.
    let initial_balance = client.free_balance(&author.public()).await.unwrap();

    let unregister_user_message = message::UnregisterUser {
        user_id: user_id.clone(),
    };
    let random_fee = random_balance();
    let tx_unregister_applied = submit_ok_with_fee(
        &client,
        &author,
        unregister_user_message.clone(),
        random_fee,
    )
    .await;
    assert_eq!(
        tx_unregister_applied.result,
        Err(RegistryError::UnregisterableUser.into())
    );
    assert!(
        user_exists(&client, unregister_user_message.user_id.clone()).await,
        "The user was expected to still exist"
    );
    assert_eq!(
        client.free_balance(&author.public()).await.unwrap(),
        initial_balance - random_fee,
        "The tx fee was not charged properly."
    );
}

#[async_std::test]
async fn unregister_user_with_invalid_sender() {
    let (client, _) = Client::new_emulator();
    let (_, user_id) = key_pair_with_associated_user(&client).await;

    // Invalid unregistration.
    let (bad_actor, _) = key_pair_with_associated_user(&client).await;
    let unregister_user_message = message::UnregisterUser {
        user_id: user_id.clone(),
    };
    let tx_unregister_applied =
        submit_ok(&client, &bad_actor, unregister_user_message.clone()).await;

    assert_eq!(
        tx_unregister_applied.result,
        Err(RegistryError::InsufficientSenderPermissions.into())
    );
    assert!(
        user_exists(&client, user_id.clone()).await,
        "The user was expected to exist"
    );
}

#[async_std::test]
async fn unregister_user_with_no_associated_user() {
    let (client, _) = Client::new_emulator();
    let alice = key_pair_from_string("Alice");
    let initial_balance = client.free_balance(&alice.public()).await.unwrap();
    let unregister_user_message = message::UnregisterUser {
        user_id: random_id(),
    };

    assert!(
        !user_exists(&client, unregister_user_message.user_id.clone()).await,
        "User should not exist",
    );

    let random_fee = random_balance();
    let tx_unregister_applied =
        submit_ok_with_fee(&client, &alice, unregister_user_message.clone(), random_fee).await;
    assert_eq!(
        tx_unregister_applied.result,
        Err(RegistryError::InexistentUser.into())
    );

    assert_eq!(
        client.free_balance(&alice.public()).await.unwrap(),
        initial_balance - random_fee,
        "The tx fee was not charged properly."
    );
}
