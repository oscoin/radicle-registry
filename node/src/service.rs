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

//! Functions to create [sc_service::Service] values used for commands.
//!
//! This module is based on `service` module from the Substrate node template.

use futures::StreamExt;
use std::convert::TryFrom;
use std::time::Duration;

use sc_client_api::client::BlockchainEvents as _;
use sc_consensus::LongestChain;
use sc_executor::native_executor_instance;
use sc_service::{AbstractService, Configuration, Error};
use sp_inherents::InherentDataProviders;

use radicle_registry_runtime::{registry::AuthoringInherentData, AccountId, RuntimeApi};

use crate::blockchain::Block;
use crate::metrics::register_metrics;
use crate::pow::{blake3_pow::Blake3Pow, config::Config, dummy_pow::DummyPow};

pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;

native_executor_instance!(
        pub Executor,
        radicle_registry_runtime::api::dispatch,
        radicle_registry_runtime::native_version,
);

/// Starts a `ServiceBuilder` for a full service.
macro_rules! new_full_start {
    ($config:expr, $inherent_data_providers: expr) => {{
        let mut import_setup = None;
        let builder = sc_service::ServiceBuilder::new_full::<Block, RuntimeApi, Executor>($config)?
            .with_select_chain(|_config, backend| Ok(LongestChain::new(backend.clone())))?
            .with_transaction_pool(|builder| {
                let pool_api = sc_transaction_pool::FullChainApi::new(builder.client().clone());
                Ok(sc_transaction_pool::BasicPool::new(
                    builder.config().transaction_pool.clone(),
                    std::sync::Arc::new(pool_api),
                    builder.prometheus_registry(),
                ))
            })?
            .with_import_queue(
                |config, client, select_chain, _transaction_pool, spawn_task_handle, _registry| {
                    let (block_import, import_queue) = node_import_queue!(
                        config,
                        client,
                        select_chain,
                        $inherent_data_providers.clone(),
                        spawn_task_handle
                    );
                    import_setup = Some(block_import);
                    Ok(import_queue)
                },
            )?;

        (builder, import_setup)
    }};
}

/// Start mining on full node
macro_rules! start_mine {
    ($block_import:expr, $service:expr, $proposer:expr, $inherent_data_providers:expr, $pow_alg:expr) => {{
        sc_consensus_pow::start_mine(
            $block_import,
            $service.client(),
            $pow_alg,
            $proposer,
            None,
            0,
            $service.network(),
            Duration::new(2, 0),
            $service.select_chain(),
            $inherent_data_providers,
            sp_consensus::AlwaysCanAuthor,
        );
    }};
}

/// The node with_import_queue closure body
macro_rules! node_import_queue {
    ($config:expr, $client:expr, $select_chain:expr, $inherent_data_providers:expr, $spawner:expr) => {{
        match Config::try_from($config)? {
            Config::Dummy => node_import_queue_for_pow_alg!(
                $client,
                $select_chain,
                $inherent_data_providers,
                DummyPow,
                $spawner
            ),
            Config::Blake3 => node_import_queue_for_pow_alg!(
                $client,
                $select_chain,
                $inherent_data_providers,
                Blake3Pow::new($client.clone()),
                $spawner
            ),
        }
    }};
}

/// The node with_import_queue closure body when PoW algorithm is known
macro_rules! node_import_queue_for_pow_alg {
    ($client:expr, $select_chain:expr, $inherent_data_providers:expr, $pow_alg:expr, $spawner:expr) => {{
        let pow_block_import = sc_consensus_pow::PowBlockImport::new(
            $client.clone(),
            $client.clone(),
            $pow_alg,
            0,
            $select_chain,
            $inherent_data_providers,
        );
        let block_import_box = Box::new(pow_block_import);
        let import_queue = sc_consensus_pow::import_queue(
            block_import_box.clone(),
            None,
            None,
            $pow_alg,
            $inherent_data_providers,
            $spawner,
            None,
        )?;
        let block_import = block_import_box as sp_consensus::import_queue::BoxBlockImport<_, _>;
        (block_import, import_queue)
    }};
}

/// Builds a new service for a full client.
///
/// Starts a miner if `opt_block_author` was provided.
pub fn new_full(
    config: Configuration,
    opt_block_author: Option<AccountId>,
) -> Result<impl AbstractService, Error> {
    log::info!(
        "Native runtime version: spec={} impl={}",
        radicle_registry_runtime::VERSION.spec_version,
        radicle_registry_runtime::VERSION.impl_version,
    );

    let pow_alg = Config::try_from(&config)?;
    let inherent_data_providers = InherentDataProviders::new();
    let (builder, import_setup) = new_full_start!(config, inherent_data_providers.clone());
    let block_import = import_setup.expect("No import setup set for miner");

    let service = builder.build_full()?;
    register_metrics(&service, service.client())?;

    if let Some(block_author) = opt_block_author {
        let client = service.client();
        service.spawn_essential_task_handle().spawn(
            "mined-block-notifier",
            client.import_notification_stream().for_each(|info| {
                if info.origin == sp_consensus::BlockOrigin::Own {
                    log::info!("Imported own block #{} ({})", info.header.number, info.hash)
                }
                futures::future::ready(())
            }),
        );

        let authoring_inherent_data = AuthoringInherentData { block_author };

        // Can only fail if a provider with the same name is already registered.
        inherent_data_providers
            .register_provider(authoring_inherent_data)
            .unwrap();

        let proposer = sc_basic_authorship::ProposerFactory::new(
            service.client(),
            service.transaction_pool(),
            service.prometheus_registry().as_ref(),
        );

        log::info!("Starting block miner");

        match pow_alg {
            Config::Dummy => start_mine!(
                block_import,
                service,
                proposer,
                inherent_data_providers,
                DummyPow
            ),
            Config::Blake3 => start_mine!(
                block_import,
                service,
                proposer,
                inherent_data_providers,
                Blake3Pow::new(client)
            ),
        }
    } else {
        log::info!("Mining is disabled");
    }

    Ok(service)
}

/// Build a new service to be used for one-shot commands.
pub fn new_for_command(
    config: Configuration,
) -> Result<impl sc_service::ServiceBuilderCommand<Block = Block>, Error> {
    let inherent_data_providers = InherentDataProviders::new();
    Ok(new_full_start!(config, inherent_data_providers).0)
}
