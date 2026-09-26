use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{MAX_TIMESTAMP_MS, Ulid};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerateError {
    ClockBeforeEpoch,
    TimestampOutOfRange,
    ClockRegression,
    RandomnessExhausted,
    EntropyUnavailable,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ClockBeforeEpoch => "clock is before the Unix epoch",
            Self::TimestampOutOfRange => "timestamp exceeds 48 bits",
            Self::ClockRegression => "clock moved backwards",
            Self::RandomnessExhausted => "80-bit monotonic randomness exhausted",
            Self::EntropyUnavailable => "operating system entropy unavailable",
        })
    }
}
impl std::error::Error for GenerateError {}

/// Monotonic within this generator. Share behind a mutex for concurrent use.
/// No global state; separate generators do not promise a total order.
#[derive(Default)]
pub struct Generator {
    last: Option<Ulid>,
}

impl Generator {
    pub const fn new() -> Self {
        Self { last: None }
    }

    /// Uses the system clock and OS entropy. Never falls back to weak randomness.
    pub fn generate(&mut self) -> Result<Ulid, GenerateError> {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| GenerateError::ClockBeforeEpoch)?
            .as_millis();
        let millis = u64::try_from(millis).map_err(|_| GenerateError::TimestampOutOfRange)?;
        self.generate_with(millis, |bytes| {
            getrandom::fill(bytes).map_err(|_| GenerateError::EntropyUnavailable)
        })
    }

    /// Deterministic seam for tests and host-owned clock/entropy adapters.
    /// The callback must supply secure randomness in production. Equal timestamps
    /// increment the random component; clock regression and overflow fail without
    /// advancing state. Entropy is requested only for a new timestamp.
    pub fn generate_with(
        &mut self,
        timestamp_ms: u64,
        entropy: impl FnOnce(&mut [u8; 10]) -> Result<(), GenerateError>,
    ) -> Result<Ulid, GenerateError> {
        if timestamp_ms > MAX_TIMESTAMP_MS {
            return Err(GenerateError::TimestampOutOfRange);
        }
        if let Some(last) = self.last {
            if timestamp_ms < last.timestamp_ms() {
                return Err(GenerateError::ClockRegression);
            }
            if timestamp_ms == last.timestamp_ms() {
                if last.0 & ((1_u128 << 80) - 1) == (1_u128 << 80) - 1 {
                    return Err(GenerateError::RandomnessExhausted);
                }
                let next = Ulid(last.0 + 1);
                self.last = Some(next);
                return Ok(next);
            }
        }
        let mut bytes = [0; 10];
        entropy(&mut bytes)?;
        let next = Ulid::from_parts(timestamp_ms, bytes)?;
        self.last = Some(next);
        Ok(next)
    }
}
