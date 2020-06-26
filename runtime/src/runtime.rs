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

use alloc::prelude::v1::*;
use frame_support::{construct_runtime, parameter_types, weights::Weight};
use frame_system as system;
use radicle_registry_core::{state::AccountTransactionIndex, Balance};
use sp_runtime::{traits::Block as BlockT, Perbill};
use sp_timestamp::OnTimestampSet;
use sp_version::RuntimeVersion;

use crate::{
    registry, timestamp_in_digest, AccountId, Block, BlockNumber, Hash, Hashing, Header, Moment,
    UncheckedExtrinsic, VERSION,
};

pub mod api;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const MaximumBlockWeight: Weight = 1_000_000_000_000;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub const Version: RuntimeVersion = VERSION;
}

impl frame_system::Trait for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = sp_runtime::traits::IdentityLookup<AccountId>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = AccountTransactionIndex;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = Hashing;
    /// The header type.
    type Header = Header;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Maximum weight of each block. With a default weight system of 1byte == 1weight, 4mb is ok.
    type MaximumBlockWeight = MaximumBlockWeight;
    /// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
    type MaximumBlockLength = MaximumBlockLength;
    /// Portion of the block weight that is available to all normal transactions.
    type AvailableBlockRatio = AvailableBlockRatio;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = ();
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type ModuleToIndex = ModuleToIndex;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = Balances;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
    /// Minimum time between blocks in milliseconds
    pub const MinimumPeriod: Moment = 300;
}

impl pallet_timestamp::Trait for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = StoreTimestampInDigest;
    type MinimumPeriod = MinimumPeriod;
}

pub struct StoreTimestampInDigest;

impl OnTimestampSet<Moment> for StoreTimestampInDigest {
    fn on_timestamp_set(timestamp: Moment) {
        let item = timestamp_in_digest::digest_item(timestamp);
        frame_system::Module::<Runtime>::deposit_log(item);
    }
}

parameter_types! {
    /// The minimum amount required to keep an account open.
    /// Transfers leaving the recipient with less than this
    /// value fail.
    pub const ExistentialDeposit: u128 = 1;
    pub const TransferFee: u128 = 0;
    pub const CreationFee: u128 = 0;
    pub const TransactionBaseFee: u128 = 0;
    pub const TransactionByteFee: u128 = 0;
}

impl pallet_balances::Trait for Runtime {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

impl pallet_sudo::Trait for Runtime {
    type Event = Event;
    type Call = Call;
}

impl registry::Trait for Runtime {
    type Event = Event;
}

construct_runtime!(
        pub enum Runtime where
                Block = Block,
                NodeBlock = Block,
                UncheckedExtrinsic = UncheckedExtrinsic
        {
                System: system::{Module, Call, Storage, Config, Event<T>},
                Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
                RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
                Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
                Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
                Registry: registry::{Module, Call, Storage, Event, Inherent},
        }
);
