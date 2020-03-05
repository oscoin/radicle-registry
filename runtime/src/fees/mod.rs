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

use crate::Balance;
use frame_support::traits::WithdrawReason;

mod bid;
mod payment;

pub trait Fee {
    fn value(&self) -> Balance;
    fn withdraw_reason(&self) -> WithdrawReason;
}

#[derive(Clone, Debug)]
pub struct BaseFee;
impl Fee for BaseFee {
    fn value(&self) -> Balance {
        1
    }

    fn withdraw_reason(&self) -> WithdrawReason {
        WithdrawReason::TransactionPayment
    }
}

#[derive(Clone, Debug)]
pub struct Tip(Balance);
impl Fee for Tip {
    fn value(&self) -> Balance {
        self.0
    }

    fn withdraw_reason(&self) -> WithdrawReason {
        WithdrawReason::Tip
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::Rng;


    #[test]
    fn withdraw_reason() {
        assert_eq!(BaseFee{}.withdraw_reason(), WithdrawReason::TransactionPayment);
        assert_eq!(Tip(123).withdraw_reason(), WithdrawReason::Tip);
    }

    #[test]
    fn base_fee_value() {
        assert_eq!(BaseFee{}.value(), 1);
    }

    #[test]
    fn tip_value() {
        for _ in 0 .. 50 {
            let random_tip: Balance = rand::thread_rng().gen();
            assert_eq!(Tip(random_tip).value(), random_tip);
        }
    }
}