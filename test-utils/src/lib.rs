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

//! Miscellaneous helpers used throughout Registry tests.

use std::convert::TryFrom;

use rand::distributions::Alphanumeric;
use rand::Rng;

use radicle_registry_client::*;

/// Submit a transaction and wait for it to be successfully applied.
///
/// Panics if submission errors.
pub async fn submit_ok_with_fee<Message_: Message>(
    client: &Client,
    author: &ed25519::Pair,
    message: Message_,
    fee: Balance,
) -> TransactionIncluded<Message_> {
    client
        .sign_and_submit_message(&author, message, fee)
        .await
        .unwrap()
        .await
        .unwrap()
}

/// Submit a transaction and wait for it to be successfully applied.
///
/// Panics if submission errors.
pub async fn submit_ok<Message_: Message>(
    client: &Client,
    author: &ed25519::Pair,
    message: Message_,
) -> TransactionIncluded<Message_> {
    submit_ok_with_fee(&client, &author, message, random_balance()).await
}

pub async fn create_project_with_checkpoint(
    domain: &ProjectDomain,
    client: &Client,
    author: &ed25519::Pair,
) -> Project {
    let checkpoint_id = submit_ok(
        &client,
        &author,
        message::CreateCheckpoint {
            project_hash: H256::random(),
            previous_checkpoint_id: None,
        },
    )
    .await
    .result
    .unwrap();

    let domain_account = match domain {
        ProjectDomain::Org(org_id) => {
            let register_org_message = message::RegisterOrg {
                org_id: org_id.clone(),
            };
            submit_ok(&client, &author, register_org_message).await;
            let org = client.get_org(org_id.clone()).await.unwrap().unwrap();
            org.account_id
        }
        ProjectDomain::User(user_id) => {
            let register_user_message = message::RegisterUser {
                user_id: user_id.clone(),
            };
            submit_ok(&client, &author, register_user_message).await;
            let user = client.get_user(user_id.clone()).await.unwrap().unwrap();
            user.account_id
        }
    };

    // The domain account needs funds to submit transactions.
    transfer(&client, &author, domain_account, 1000).await;

    let register_project_message = random_register_project_message(domain, checkpoint_id);
    submit_ok(&client, &author, register_project_message.clone()).await;

    client
        .get_project(register_project_message.project_name, domain.clone())
        .await
        .unwrap()
        .unwrap()
}

pub async fn create_random_org(client: &Client, author: &ed25519::Pair) -> Org {
    let register_org_message = random_register_org_message();
    submit_ok(&client, &author, register_org_message.clone()).await;

    client
        .get_org(register_org_message.org_id)
        .await
        .unwrap()
        .unwrap()
}

pub fn random_id() -> Id {
    let size = rand::thread_rng().gen_range(1, 33);
    Id::try_from(random_alnum_string(size).to_lowercase()).unwrap()
}

pub fn random_project_name() -> ProjectName {
    let size = rand::thread_rng().gen_range(1, 33);
    ProjectName::try_from(random_alnum_string(size).to_lowercase()).unwrap()
}

/// Create a [message::RegisterOrg] with random parameters.
pub fn random_register_org_message() -> message::RegisterOrg {
    message::RegisterOrg {
        org_id: random_id(),
    }
}

/// Create a [message::RegisterProject] with random parameters to register a project with.
pub fn random_register_project_message(
    domain: &ProjectDomain,
    checkpoint_id: CheckpointId,
) -> message::RegisterProject {
    message::RegisterProject {
        project_name: random_project_name(),
        project_domain: domain.clone(),
        checkpoint_id,
        metadata: Bytes128::random(),
    }
}

/// Create a [message::RegisterUser] with random parameters.
pub fn random_register_user_message() -> message::RegisterUser {
    message::RegisterUser {
        user_id: random_id(),
    }
}

pub fn key_pair_from_string(value: impl AsRef<str>) -> ed25519::Pair {
    ed25519::Pair::from_string(format!("//{}", value.as_ref()).as_str(), None).unwrap()
}

/// Create a key pair derived from `value` and register a user with that key pair.
/// The user name is `value.to_lowercase()`. Ensures that the account for the key
/// pair is equipped with at least 100.000 RAD.
pub async fn key_pair_with_associated_user(client: &Client) -> (ed25519::Pair, Id) {
    let seed = &random_alnum_string(5);
    let key_pair = key_pair_from_string(seed);

    // Have Alice transfer 100.000 μRAD to this new account, necessary to submit transactions.
    let alice = key_pair_from_string("Alice");
    transfer(&client, &alice, key_pair.public(), 100_000).await;

    let user_id = Id::try_from(seed.to_lowercase()).unwrap();
    let register_user_message = message::RegisterUser {
        user_id: user_id.clone(),
    };
    let tx_applied = submit_ok(&client, &key_pair, register_user_message).await;
    assert_eq!(tx_applied.result, Ok(()));

    (key_pair, user_id)
}

pub fn random_alnum_string(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .collect::<String>()
}

/// Check if the user with the given id exists in the chain state.
pub async fn user_exists(client: &Client, user_id: Id) -> bool {
    client
        .list_users()
        .await
        .unwrap()
        .iter()
        .any(|id| *id == user_id.clone())
}

pub fn random_balance() -> Balance {
    rand::thread_rng().gen_range(20, 100)
}

pub async fn transfer(
    client: &Client,
    donator: &ed25519::Pair,
    recipient: AccountId,
    value: Balance,
) {
    let tx_included = submit_ok_with_fee(
        &client,
        &donator,
        message::Transfer {
            recipient,
            balance: value,
        },
        1,
    )
    .await;
    assert_eq!(
        tx_included.result,
        Ok(()),
        "Failed to grant funds to the recipient account."
    );
}
