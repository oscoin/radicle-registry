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

/// `String32` validation tests.
use alloc::format;
use alloc::prelude::v1::*;
use parity_scale_codec::{Decode, Encode, Error as CodecError, Input};

use sr_std::fmt;
use sr_std::str::FromStr;

/// Type to represent project names and domains.
///
/// Since their lengths are limited to 32 characters, a smart constructor is
/// provided to check validity.
#[derive(Encode, Clone, Debug, Eq, PartialEq)]
pub struct String32(String);

impl String32 {
    pub fn from_string(s: String) -> Result<Self, String> {
        if s.len() > 32 {
            Err(format!(
                "The provided string's length exceeded 32 characters: {:?}",
                s
            ))
        } else {
            Ok(String32(s))
        }
    }
}

impl FromStr for String32 {
    type Err = String;

    /// This function only raises an error if the `String` it is passed is
    /// longer than 32 characters.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        String32::from_string(s.to_string())
    }
}

#[cfg(feature = "std")]
impl fmt::Display for String32 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Decode for String32 {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let decoded: String = String::decode(input)?;
        if decoded.len() > 32 {
            Err(From::from("String32 length was more than 32 characters."))
        } else {
            Ok(String32(decoded))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn long_string32() {
        fn long_string(n: usize) -> Result<String32, String> {
            String32::from_string(std::iter::repeat("X").take(n).collect::<String>())
        }
        let wrong = long_string(33);
        let right = long_string(32);

        assert!(
            wrong.is_err(),
            "Error: excessively long string converted to String32"
        );
        assert!(
            right.is_ok(),
            "Error: string with acceptable length failed conversion to String32."
        )
    }

    #[test]
    fn encode_then_decode() {
        let string = String32::from_string(String::from("ôítÏйгますいщαφδвы")).unwrap();

        let encoded = string.encode();

        let decoded = <String32>::decode(&mut &encoded[..]).unwrap();

        assert_eq!(string, decoded)
    }
}
