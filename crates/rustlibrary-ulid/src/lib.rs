//! Typed ULIDs with canonical text and fallible, explicitly owned generation.
//! IDs are probabilistically unique identifiers, never access tokens.

use std::{fmt, str::FromStr};

mod generator;
pub use generator::{GenerateError, Generator};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
pub const MAX_TIMESTAMP_MS: u64 = (1_u64 << 48) - 1;

/// A 48-bit timestamp and 80-bit random component in network byte order.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ulid(u128);

impl Ulid {
    /// Constructs a ULID without consulting clock or entropy sources.
    pub fn from_parts(timestamp_ms: u64, randomness: [u8; 10]) -> Result<Self, GenerateError> {
        if timestamp_ms > MAX_TIMESTAMP_MS {
            return Err(GenerateError::TimestampOutOfRange);
        }
        let mut bytes = [0; 16];
        bytes[..6].copy_from_slice(&timestamp_ms.to_be_bytes()[2..]);
        bytes[6..].copy_from_slice(&randomness);
        Ok(Self::from_bytes(bytes))
    }

    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(u128::from_be_bytes(bytes))
    }

    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }

    pub const fn timestamp_ms(self) -> u64 {
        (self.0 >> 80) as u64
    }

    /// Canonical uppercase ASCII representation, without allocation.
    pub fn encode(self) -> [u8; 26] {
        let mut output = [0; 26];
        let mut bits = self.0;
        for slot in output.iter_mut().rev() {
            *slot = ALPHABET[(bits & 31) as usize];
            bits >>= 5;
        }
        output
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.encode() {
            write!(f, "{}", char::from(byte))?;
        }
        Ok(())
    }
}

impl fmt::Debug for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ulid({self})")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    Length,
    InvalidCharacter { index: usize },
    Overflow,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length => f.write_str("ULID must contain exactly 26 ASCII characters"),
            Self::InvalidCharacter { index } => write!(f, "invalid ULID character at byte {index}"),
            Self::Overflow => f.write_str("ULID exceeds 128 bits"),
        }
    }
}
impl std::error::Error for ParseError {}

impl FromStr for Ulid {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 26 {
            return Err(ParseError::Length);
        }
        let mut bits = 0_u128;
        for (index, byte) in text.bytes().enumerate() {
            let digit = ALPHABET
                .iter()
                .position(|value| *value == byte.to_ascii_uppercase())
                .ok_or(ParseError::InvalidCharacter { index })?;
            if index == 0 && digit > 7 {
                return Err(ParseError::Overflow);
            }
            bits = (bits << 5) | digit as u128;
        }
        Ok(Self(bits))
    }
}
