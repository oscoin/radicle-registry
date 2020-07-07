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

//! Provides [Cli] struct that represents the command line arguments.
use radicle_registry_runtime::AccountId;
use sc_cli::{RunCmd, Subcommand, SubstrateCli};
use sc_network::config::MultiaddrWithPeerId;
use sc_service::{ChainSpec, Configuration};
use std::path::PathBuf;
use std::sync::Arc;
use structopt::StructOpt;

use crate::service;

lazy_static::lazy_static! {
    static ref DEFAULT_CHAIN: &'static str = option_env!("DEFAULT_CHAIN").unwrap_or("dev");
}

/// Full node for the Radicle Registry network
#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    subcommand: Option<Subcommand>,

    /// Chain to connect to.
    #[structopt(
        long,
        default_value = &DEFAULT_CHAIN,
        value_name = "CHAIN",
        possible_values = &["dev", "local-devnet", "devnet", "ffnet"]
    )]
    chain: String,

    /// Bind the RPC HTTP and WebSocket APIs to `0.0.0.0` instead of the local interface.
    #[structopt(long)]
    unsafe_rpc_external: bool,

    /// List of nodes to connect to on start.
    /// The addresses must be expressed as libp2p multiaddresses with a peer ID, e.g.
    /// `/ip4/35.233.120.254/tcp/30333/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR`.
    /// For more information visit https://docs.libp2p.io/concepts/addressing/
    #[structopt(long, short, value_name = "ADDR")]
    bootnodes: Vec<MultiaddrWithPeerId>,

    /// Where to store data
    #[structopt(long, short, value_name = "PATH")]
    data_path: Option<std::path::PathBuf>,

    /// The secret key to use for libp2p networking provided as a hex-encoded Ed25519 32 bytes
    /// secret key.
    ///
    /// The value of this option takes precedence over `--node-key-file`.
    ///
    /// WARNING: Secrets provided as command-line arguments are easily exposed.
    /// Use of this option should be limited to development and testing. To use
    /// an externally managed secret key, use `--node-key-file` instead.
    #[structopt(long, value_name = "HEX_KEY")]
    node_key: Option<String>,

    /// The file from which to read the node's secret key to use for libp2p networking.
    ///
    /// The file must contain an unencoded 32 bytes Ed25519 secret key.
    ///
    /// If the file does not exist, it is created with a newly generated secret key.
    #[structopt(long, value_name = "FILE")]
    node_key_file: Option<std::path::PathBuf>,

    /// Enable mining and credit rewards to the given account.
    ///
    /// The account address must be given in SS58 format.
    #[structopt(long, value_name = "SS58_ADDRESS", parse(try_from_str = parse_ss58_account_id))]
    mine: Option<AccountId>,

    /// Bind the prometheus metrics endpoint to 0.0.0.0 on port 9615
    #[structopt(long)]
    prometheus_external: bool,

    /// Human-readable name for this node to use for telemetry
    #[structopt(long, value_name = "NAME")]
    name: Option<String>,

    /// Disable sending telemetry data to https://telemetry.polkadot.io/
    #[structopt(long)]
    no_telemetry: bool,

    /// Specify path to a JSON with a chain spec to use
    #[structopt(long, conflicts_with = "chain")]
    spec: Option<PathBuf>,

    /// Run the dev chain with an in-memory database and mining
    #[structopt(long, conflicts_with = "chain")]
    dev: bool,
}

impl SubstrateCli for Cli {
    fn impl_name() -> &'static str {
        "Radicle Registry Node"
    }

    fn impl_version() -> &'static str {
        // uses `git describe`
        env!("VERGEN_SEMVER")
    }

    fn description() -> &'static str {
        env!("CARGO_PKG_DESCRIPTION")
    }

    fn author() -> &'static str {
        env!("CARGO_PKG_AUTHORS")
    }

    fn support_url() -> &'static str {
        "http://github.com/radicle-dev/radicle-registry/issues"
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn executable_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpec>, String> {
        if let Some(spec_path) = &self.spec {
            crate::chain_spec::from_spec_file(spec_path.clone())
        } else {
            match id {
                "dev" => Ok(crate::chain_spec::dev()),
                "local-devnet" => Ok(crate::chain_spec::local_devnet()),
                "devnet" => Ok(crate::chain_spec::devnet()),
                "ffnet" => Ok(crate::chain_spec::ffnet()),
                other => Err(format!("Invalid chain {}", other)),
            }
        }
        .map(|chain_spec| Box::new(chain_spec) as _)
    }
}

impl Cli {
    pub fn run(&self) -> sc_cli::Result<()> {
        crate::logger::init();
        match &self.subcommand {
            Some(subcommand) => {
                let result = self
                    .create_runner(subcommand)?
                    .run_subcommand(subcommand, |config| {
                        service::new_for_command(self.adjust_config(config))
                    });
                // Workaround until substrate is updated and
                // https://github.com/paritytech/substrate/pull/6098 is included.
                use std::io::Write;
                let _ = std::io::stdout().flush();
                result
            }
            None => self.create_runner(&self.create_run_cmd())?.run_node(
                |_config| {
                    // This should never be called since it is not accesible via the command line.
                    panic!("Light client support not implemented");
                    // We leave this call here so that the type checker can properly infer the type
                    // of this closure.
                    #[allow(unreachable_code)]
                    service::new_full(self.adjust_config(_config), self.block_author())
                },
                |config| service::new_full(self.adjust_config(config), self.block_author()),
                radicle_registry_runtime::VERSION,
            ),
        }
    }

    fn block_author(&self) -> Option<AccountId> {
        if let Some(block_author) = self.mine {
            Some(block_author)
        } else if self.dev {
            use sp_core::crypto::Pair;
            let block_author = sp_core::ed25519::Pair::from_string("//Mine", None).unwrap();
            Some(block_author.public())
        } else {
            None
        }
    }

    fn create_run_cmd(&self) -> RunCmd {
        // This does not panic if there are no required arguments which we statically know.
        let mut run_cmd = RunCmd::from_iter_safe(vec![] as Vec<String>).unwrap();
        run_cmd.no_telemetry = self.no_telemetry;
        run_cmd.shared_params.chain = if self.dev {
            Some(String::from("dev"))
        } else {
            Some(self.chain.clone())
        };
        run_cmd.network_params.bootnodes = self.bootnodes.clone();
        run_cmd.network_params.node_key_params.node_key = self.node_key.clone();
        run_cmd.network_params.node_key_params.node_key_file = self.node_key_file.clone();
        run_cmd.shared_params.base_path = self.data_path.clone();
        run_cmd.unsafe_rpc_external = self.unsafe_rpc_external;
        run_cmd.unsafe_ws_external = self.unsafe_rpc_external;
        run_cmd.prometheus_external = self.prometheus_external;
        run_cmd.name = self.name.clone();
        run_cmd
    }

    /// Applies CLI settings from `self` to the configuration.
    fn adjust_config(&self, mut config: Configuration) -> Configuration {
        use sc_chain_spec::ChainType;
        use sc_client_api::{execution_extensions::ExecutionStrategies, ExecutionStrategy};

        let execution_strategy = if self.dev && self.spec.is_some() {
            ExecutionStrategy::AlwaysWasm
        } else {
            match config.chain_spec.chain_type() {
                // During development we want to run a node that runs a changed runtime without having
                // to recompile the genesis WASM runtime.
                ChainType::Development => ExecutionStrategy::NativeWhenPossible,
                _ => ExecutionStrategy::Both,
            }
        };

        config.execution_strategies = ExecutionStrategies {
            syncing: execution_strategy,
            importing: execution_strategy,
            block_construction: execution_strategy,
            offchain_worker: execution_strategy,
            other: execution_strategy,
        };

        if self.dev {
            let db = Arc::new(sp_database::MemDb::new());
            config.database = sc_service::config::DatabaseConfig::Custom(db);
            config.network.transport = sc_network::config::TransportConfig::MemoryOnly;
        }

        if self.unsafe_rpc_external {
            config.rpc_cors = None;
        }
        config
    }
}

fn parse_ss58_account_id(data: &str) -> Result<AccountId, String> {
    sp_core::crypto::Ss58Codec::from_ss58check(data).map_err(|err| format!("{:?}", err))
}
