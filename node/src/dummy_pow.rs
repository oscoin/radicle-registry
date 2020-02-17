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

use radicle_registry_runtime::opaque::Block;
use radicle_registry_runtime::Hash;
use sc_consensus_pow::{Error, PowAlgorithm};
use sp_consensus_pow::Seal;
use sp_runtime::generic::BlockId;

/// This is a dummy implementation of a PoW algorithm that doesn't do anything and
/// **provides no security**. Do not use it outside of a tightly controlled devnet!
///
/// It produces no seals. The mining process consists of
/// a random length sleep and successful returning of a 0-byte nonce.
///
/// It accepts all seals. Verification is always successful.
#[derive(Clone)]
pub struct DummyPow;

impl PowAlgorithm<Block> for DummyPow {
    type Difficulty = u128;

    fn difficulty(&self, _parent: &BlockId<Block>) -> Result<Self::Difficulty, Error<Block>> {
        Ok(1)
    }

    fn verify(
        &self,
        _parent: &BlockId<Block>,
        _pre_hash: &Hash,
        _seal: &Seal,
        _difficulty: Self::Difficulty,
    ) -> Result<bool, Error<Block>> {
        Ok(true)
    }

    fn mine(
        &self,
        _parent: &BlockId<Block>,
        _pre_hash: &Hash,
        _difficulty: Self::Difficulty,
        _round: u32,
    ) -> Result<Option<Seal>, Error<Block>> {
        std::thread::sleep(std::time::Duration::from_millis(10));
        if rand::random::<f32>() < 0.005 {
            Ok(Some(vec![]))
        } else {
            Ok(None)
        }
    }
}
