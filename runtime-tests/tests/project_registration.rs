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

// Verify that a project can be registered under a user and an org.
// Note that this also tests that a project with the same name can coexist
// under those two different domains.
#[async_std::test]
async fn register_project() {
    let (client, _) = Client::new_emulator();
    let author = key_pair_with_funds(&client).await;

    for domain in generate_project_domains(&client, &author).await {
        let initial_balance = match &domain {
            ProjectDomain::Org(org_id) => {
                let org = client.get_org(org_id.clone()).await.unwrap().unwrap();
                client.free_balance(&org.account_id()).await.unwrap()
            }
            ProjectDomain::User(user_id) => {
                let user = client.get_user(user_id.clone()).await.unwrap().unwrap();
                client.free_balance(&user.account_id()).await.unwrap()
            }
        };

        let random_fee = random_balance();
        let message = random_register_project_message(&domain);
        let project_name = message.project_name.clone();
        let tx_included = submit_ok_with_fee(&client, &author, message.clone(), random_fee).await;
        assert_eq!(tx_included.result, Ok(()));

        let project = client
            .get_project(message.project_name.clone(), message.project_domain.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project.metadata().clone(), message.metadata.clone());

        let has_project = client
            .list_projects()
            .await
            .unwrap()
            .iter()
            .any(|id| *id == (message.project_name.clone(), message.project_domain.clone()));
        assert!(has_project, "Registered project not found in project list");

        let (projects, account_id) = match &domain {
            ProjectDomain::Org(org_id) => {
                let org = client.get_org(org_id.clone()).await.unwrap().unwrap();
                (org.projects().clone(), org.account_id())
            }
            ProjectDomain::User(user_id) => {
                let user = client.get_user(user_id.clone()).await.unwrap().unwrap();
                (user.projects().clone(), user.account_id())
            }
        };

        assert_eq!(projects, vec![project_name]);
        assert_eq!(
            client.free_balance(&account_id).await.unwrap(),
            initial_balance - random_fee,
            "The tx fee was not charged properly."
        );
    }
}

// Verify that a project can not be registered under a domain that does not exist.
#[async_std::test]
async fn register_project_under_inexistent_domain() {
    let (client, _) = Client::new_emulator();
    let (author, _) = key_pair_with_associated_user(&client).await;

    for domain in vec![
        ProjectDomain::Org(random_id()),
        ProjectDomain::User(random_id()),
    ] {
        let message = random_register_project_message(&domain);
        let tx_included = submit_ok(&client, &author, message.clone()).await;

        let expected_error = match domain {
            ProjectDomain::Org(_) => RegistryError::InexistentOrg,
            ProjectDomain::User(_) => RegistryError::InexistentUser,
        };
        assert_eq!(tx_included.result, Err(expected_error.into()));
    }
}

// Verify that a same project can not be re-registered under the same user or org.
#[async_std::test]
async fn re_register_project_same_domain_entity() {
    let (client, _) = Client::new_emulator();
    let author = key_pair_with_funds(&client).await;

    for domain in generate_project_domains(&client, &author).await {
        let message = random_register_project_message(&domain);
        let project_name = message.project_name.clone();
        submit_ok(&client, &author, message.clone()).await;

        // Duplicate submission with a different metadata.
        let registration_2 = submit_ok(
            &client,
            &author,
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
            .get_project(message.project_name, message.project_domain)
            .await
            .unwrap()
            .unwrap();
        // Assert that the project data was not altered during the
        // attempt to re-register the already existing project.
        assert_eq!(message.metadata, *project.metadata());

        let projects_list = match &domain {
            ProjectDomain::Org(org_id) => {
                let org = client.get_org(org_id.clone()).await.unwrap().unwrap();
                org.projects().clone()
            }
            ProjectDomain::User(user_id) => {
                let user = client.get_user(user_id.clone()).await.unwrap().unwrap();
                user.projects().clone()
            }
        };

        assert_eq!(projects_list, vec![project_name]);
    }
}

// Verify that two different orgs can have a project identified by the same name.
#[async_std::test]
async fn register_same_project_name_under_different_orgs() {
    let (client, _) = Client::new_emulator();
    let (author, _) = key_pair_with_associated_user(&client).await;
    let (org_1_id, _) = register_random_org(&client, &author).await;
    let domain_org_1 = ProjectDomain::Org(org_1_id);

    let message = random_register_project_message(&domain_org_1);
    submit_ok(&client, &author, message.clone()).await;

    // Submit a project with the same name under another org.
    let (org_2_id, _) = register_random_org(&client, &author).await;
    let domain_org_2 = ProjectDomain::Org(org_2_id);
    let registration_2 = submit_ok(
        &client,
        &author,
        message::RegisterProject {
            project_domain: domain_org_2,
            ..message.clone()
        },
    )
    .await;

    assert!(registration_2.result.is_ok());
}

// Verify that two different users can have a project identified by the same name.
#[async_std::test]
async fn register_same_project_name_under_different_users() {
    let (client, _) = Client::new_emulator();
    let (author_1, user_id_1) = key_pair_with_associated_user(&client).await;
    let domain_user_1 = ProjectDomain::User(user_id_1);

    let message = random_register_project_message(&domain_user_1);
    submit_ok(&client, &author_1, message.clone()).await;

    // Duplicate submission under a different domain.
    let (author_2, user_id_2) = key_pair_with_associated_user(&client).await;
    let domain_user_2 = ProjectDomain::User(user_id_2);
    let registration_2 = submit_ok(
        &client,
        &author_2,
        message::RegisterProject {
            project_domain: domain_user_2,
            ..message.clone()
        },
    )
    .await;

    assert!(registration_2.result.is_ok());
}

// Verify that a bad author can not register projects under other users and orgs.
#[async_std::test]
async fn register_project_with_bad_actor() {
    let (client, _) = Client::new_emulator();
    let author = key_pair_with_funds(&client).await;
    let (bad_actor, _) = key_pair_with_associated_user(&client).await;

    for domain in generate_project_domains(&client, &author).await {
        // The bad actor attempts to register a project within a domain they don't belong to.
        let initial_balance = client.free_balance(&bad_actor.public()).await.unwrap();
        let register_project = random_register_project_message(&domain);
        let random_fee = random_balance();
        let tx_included =
            submit_ok_with_fee(&client, &bad_actor, register_project.clone(), random_fee).await;

        assert_eq!(
            tx_included.result,
            Err(RegistryError::InsufficientSenderPermissions.into())
        );

        // Check that the bad actor payed for the transaction anyway.
        assert_eq!(
            client.free_balance(&bad_actor.public()).await.unwrap(),
            initial_balance - random_fee,
            "The tx fee was not charged properly."
        );

        assert!(client
            .get_project(register_project.project_name, domain)
            .await
            .unwrap()
            .is_none());
    }
}
