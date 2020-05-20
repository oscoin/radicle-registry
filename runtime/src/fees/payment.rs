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

use crate::registry::{org_has_member_with_account, store};
use crate::{AccountId, Call, DispatchError, Runtime};
use radicle_registry_core::*;

use frame_support::storage::{StorageMap as _, StorageValue as _};
use frame_support::traits::{Currency, ExistenceRequirement, Imbalance, WithdrawReason};
use sp_runtime::Permill;

type NegativeImbalance = <crate::Balances as Currency<AccountId>>::NegativeImbalance;

type RegistryCall = crate::registry::Call<Runtime>;

/// Share of a transaction fee that is burned rather than credited to the block author.
const BURN_SHARE: Permill = Permill::from_percent(1);

/// Pay Fees
/// Given a tx author, their fee, and a RegistryCall they are submitting,
/// charge the tx fees to the right account, which depends on the `registry_call`.
pub fn pay(author: AccountId, fee: Balance, call: &Call) -> Result<(), DispatchError> {
    let payer = payer_account(author, call);
    let withdrawn_fee = withdraw(fee, &payer)?;
    let (burn, reward) = withdrawn_fee.split(BURN_SHARE * fee);
    drop(burn);

    // The block author is only available when this function is run as part of the block execution.
    // If this function is run as part of transaction validation the block author is not set. In
    // that case we don’t need to credit the block author.
    if let Some(block_author) = store::BlockAuthor::get() {
        crate::Balances::resolve_creating(&block_author, reward);
    }

    Ok(())
}

pub fn withdraw(fee: Balance, payer: &AccountId) -> Result<NegativeImbalance, DispatchError> {
    <crate::Balances as Currency<_>>::withdraw(
        payer,
        fee,
        WithdrawReason::TransactionPayment | WithdrawReason::Tip,
        ExistenceRequirement::KeepAlive,
    )
}

/// Find which account should pay for a given runtime call.
/// Authorize calls that involve another paying entity than the tx author.
/// The tx author pays for all unauthorized calls.
fn payer_account(author: AccountId, call: &Call) -> AccountId {
    match call {
        Call::Registry(registry_call) => match registry_call {
            // Transactions payed by the org
            RegistryCall::register_project(m) => org_payer_account(author, &m.org_id),
            RegistryCall::unregister_org(m) => org_payer_account(author, &m.org_id),
            RegistryCall::transfer_from_org(m) => org_payer_account(author, &m.org_id),
            RegistryCall::set_checkpoint(m) => org_payer_account(author, &m.org_id),
            RegistryCall::register_member(m) => org_payer_account(author, &m.org_id),

            // Transactions paid by the author
            RegistryCall::create_checkpoint(_)
            | RegistryCall::register_org(_)
            | RegistryCall::transfer(_)
            | RegistryCall::register_user(_)
            | RegistryCall::unregister_user(_) => author,

            // Inherents
            RegistryCall::set_block_author(_) => {
                panic!("Inherent calls are not allowed for signed extrinsics")
            }

            crate::registry::Call::__PhantomItem(_, _) => {
                unreachable!("__PhantomItem should never be used.")
            }
        },
        _ => author,
    }
}

/// Find which account should pay for an org-related call.
/// When the User associated with `author` is a member of the org
/// identified by `org_id`, return that org's account, otherwise the author's.
fn org_payer_account(author: AccountId, org_id: &Id) -> AccountId {
    match store::Orgs::get(org_id) {
        Some(org) => {
            if org_has_member_with_account(&org, author) {
                org.account_id
            } else {
                author
            }
        }
        None => author,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Balances, GenesisConfig};

    use core::convert::TryFrom;
    use frame_support::traits::Currency;
    use sp_core::{crypto::Pair, ed25519};
    use sp_runtime::BuildStorage;

    #[test]
    fn pay_fee() {
        let genesis_config = GenesisConfig {
            pallet_balances: None,
            pallet_sudo: None,
            system: None,
        };

        let mut test_ext = sp_io::TestExternalities::new(genesis_config.build_storage().unwrap());

        test_ext.execute_with(move || {
            let block_author = ed25519::Pair::from_string("//Bob", None).unwrap().public();
            store::BlockAuthor::put(block_author);

            let tx_author = ed25519::Pair::from_string("//Alice", None)
                .unwrap()
                .public();
            let _imbalance = Balances::deposit_creating(&tx_author, 3000);

            let fee = 1000;
            let call = RegistryCall::register_user(message::RegisterUser {
                user_id: Id::try_from("alice").unwrap(),
            })
            .into();

            pay(tx_author, fee, &call).unwrap();

            let block_author_balance = Balances::free_balance(&block_author);
            assert_eq!(block_author_balance, 990);

            let tx_author_balance = Balances::free_balance(&tx_author);
            assert_eq!(tx_author_balance, 2000)
        });
    }
}
