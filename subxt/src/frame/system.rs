//! Implements support for the frame_system module.
use crate::{
    codec::Encoded,
    error::Error,
    metadata::MetadataError,
    frame::{
        balances::Balances,
        ModuleCalls,
    },
    Client,
    Valid,
    XtBuilder,
};
use futures::future::{
    self,
    Future,
};
use parity_scale_codec::Codec;
use runtime_primitives::traits::{
    Bounded,
    CheckEqual,
    Hash,
    Header,
    IdentifyAccount,
    MaybeDisplay,
    MaybeSerialize,
    MaybeSerializeDeserialize,
    Member,
    SimpleArithmetic,
    SimpleBitOps,
    StaticLookup,
    Verify,
};
use runtime_support::Parameter;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use sp_core::Pair;

/// The subset of the `frame::Trait` that a client must implement.
pub trait System: 'static + Eq + Clone + Debug {
    /// Account index (aka nonce) type. This stores the number of previous
    /// transactions associated with a sender account.
    type Index: Parameter
        + Member
        + MaybeSerialize
        + Debug
        + Default
        + MaybeDisplay
        + SimpleArithmetic
        + Copy;

    /// The block number type used by the runtime.
    type BlockNumber: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + SimpleArithmetic
        + Default
        + Bounded
        + Copy
        + std::hash::Hash
        + sp_std::str::FromStr;

    /// The output of the `Hashing` function.
    type Hash: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + SimpleBitOps
        + Default
        + Copy
        + CheckEqual
        + std::hash::Hash
        + AsRef<[u8]>
        + AsMut<[u8]>;

    /// The hashing system (algorithm) being used in the runtime (e.g. Blake2).
    type Hashing: Hash<Output = Self::Hash>;

    /// The user account identifier type for the runtime.
    type AccountId: Parameter
        + Member
        + MaybeSerialize
        + Debug
        + MaybeDisplay
        + Ord
        + Default;

    /// The address type. This instead of `<frame_system::Trait::Lookup as StaticLookup>::Source`.
    type Address: Codec + Clone + PartialEq + Debug;

    /// The block header.
    type Header: Parameter
        + Header<Number = Self::BlockNumber, Hash = Self::Hash>
        + DeserializeOwned;

    /// Top-level event type of the runtime
    type Event: Parameter + Member;
}

/// Blanket impl for using existing runtime types
impl<T: frame_system::Trait + Debug> System for T
where
    <T as frame_system::Trait>::Header: serde::de::DeserializeOwned,
{
    type Index = T::Index;
    type BlockNumber = T::BlockNumber;
    type Hash = T::Hash;
    type Hashing = T::Hashing;
    type AccountId = T::AccountId;
    type Address = <T::Lookup as StaticLookup>::Source;
    type Header = T::Header;
    type Event = T::Event;
}

/// The System extension trait for the Client.
pub trait SystemStore {
    /// System type.
    type System: System;

    /// Returns the account nonce for an account_id.
    fn account_nonce(
        &self,
        account_id: <Self::System as System>::AccountId,
    ) -> Box<dyn Future<Item = <Self::System as System>::Index, Error = Error> + Send>;
}

impl<T: System + Balances + 'static, S: 'static> SystemStore for Client<T, S> {
    type System = T;

    fn account_nonce(
        &self,
        account_id: <Self::System as System>::AccountId,
    ) -> Box<dyn Future<Item = <Self::System as System>::Index, Error = Error> + Send>
    {
        let account_nonce_map = || {
            Ok(self
                .metadata
                .module("System")?
                .storage("AccountNonce")?
                .get_map()?)
        };
        let map = match account_nonce_map() {
            Ok(map) => map,
            Err(err) => return Box::new(future::err(err)),
        };
        Box::new(self.fetch_or(map.key(account_id), map.default()))
    }
}

/// The System extension trait for the XtBuilder.
pub trait SystemXt {
    /// System type.
    type System: System;
    /// Keypair type
    type Pair: Pair;
    /// Signature type
    type Signature: Verify;

    /// Create a call for the frame system module
    fn system<F>(
        &self,
        f: F,
    ) -> XtBuilder<Self::System, Self::Pair, Self::Signature, Valid>
    where
        F: FnOnce(
            ModuleCalls<Self::System, Self::Pair>,
        ) -> Result<Encoded, MetadataError>;
}

impl<T: System + Balances + 'static, P, S: 'static, V> SystemXt for XtBuilder<T, P, S, V>
where
    P: Pair,
    S: Verify,
    S::Signer: From<P::Public> + IdentifyAccount<AccountId = T::AccountId>,
{
    type System = T;
    type Pair = P;
    type Signature = S;

    fn system<F>(&self, f: F) -> XtBuilder<T, P, S, Valid>
    where
        F: FnOnce(
            ModuleCalls<Self::System, Self::Pair>,
        ) -> Result<Encoded, MetadataError>,
    {
        self.set_call("System", f)
    }
}

impl<T: System + 'static, P> ModuleCalls<T, P>
where
    P: Pair,
{
    /// Sets the new code.
    pub fn set_code(&self, code: Vec<u8>) -> Result<Encoded, MetadataError> {
        self.module.call("set_code", code)
    }
}

/// Event for the System module.
#[derive(Clone, Debug, parity_scale_codec::Decode)]
pub enum SystemEvent {
    /// An extrinsic completed successfully.
    ExtrinsicSuccess,
    /// An extrinsic failed.
    ExtrinsicFailed(runtime_primitives::DispatchError),
}
