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

//! Define the commands supported by the CLI.

use crate::{CommandError, CommandT, NetworkOptions, TransactionError, TxOptions};
use radicle_registry_client::*;

use sp_core::crypto::Ss58Codec;
use structopt::StructOpt;

pub mod account;
pub mod org;
pub mod other;
pub mod project;
pub mod user;

/// Check that a transaction has been applied succesfully.
///
/// Return the `Ok` value of the transaction result or, if the transaction failed
/// (i.e., `tx_applied.result` is `Err`), return a [TransactionError].
fn transaction_applied_ok<Message_, T>(
    tx_applied: &TransactionApplied<Message_>,
) -> Result<T, TransactionError>
where
    Message_: Message<Result = Result<T, DispatchError>>,
    T: Copy + Send + 'static,
{
    tx_applied
        .result
        .map_err(|dispatch_error| dispatch_error.into())
}

fn parse_account_id(data: &str) -> Result<AccountId, String> {
    Ss58Codec::from_ss58check(data).map_err(|err| format!("{:?}", err))
}

fn announce_tx(msg: &str) {
    println!("{}", msg);
    println!("⏳ Transactions might take a while to be processed. Please wait...");
}
