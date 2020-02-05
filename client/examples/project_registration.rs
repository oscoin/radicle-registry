//! Register a project on the ledger
use futures::compat::{Compat, Future01CompatExt};
use futures::future::FutureExt;

use radicle_registry_client::*;

#[async_std::main]
async fn main() {
    env_logger::init();
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(Compat::new(go().boxed())).unwrap();
    runtime.shutdown_now().compat().await.unwrap();
}

async fn go() -> Result<(), Error> {
    let alice = ed25519::Pair::from_string("//Alice", None).unwrap();

    let node_host = url::Host::parse("127.0.0.1").unwrap();
    let client = Client::create(node_host).await?;

    let project_id = ProjectId::from_string("radicle-registry".to_string()).unwrap();
    let project_org_id = OrgId::from_string("rad".to_string()).unwrap();

    // Choose some random project hash and create a checkpoint
    let project_hash = H256::random();
    let checkpoint_id = client
        .sign_and_submit_message(
            &alice,
            message::CreateCheckpoint {
                project_hash,
                previous_checkpoint_id: None,
            },
        )
        .await?
        .await?
        .result
        .unwrap();

    // Register the project
    client
        .sign_and_submit_message(
            &alice,
            message::RegisterProject {
                id: project_id.clone(),
                checkpoint_id,
                metadata: Bytes128::random(),
            },
        )
        .await?
        .await?
        .result
        .unwrap();

    println!(
        "Successfully registered project {}.{}",
        project_id, project_org_id
    );
    Ok(())
}
