//! Strict file-oriented text conversion; see README for supported encodings.

use std::fmt;
mod conversion;
mod encoding;
mod oem;
pub use encoding::{Encoding, OEM_PAGES, detect_bom};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecError {
    UnsupportedEncoding,
    MalformedInput,
    Unrepresentable,
    BomMismatch,
    UnsupportedBom,
    InputLimit,
    OutputLimit,
    AllocationFailed,
}
impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedEncoding => "unsupported or ambiguous encoding label",
            Self::MalformedInput => "invalid bytes for the selected encoding",
            Self::Unrepresentable => "text cannot be represented in the selected encoding",
            Self::BomMismatch => "Unicode signature conflicts with selected encoding",
            Self::UnsupportedBom => "selected encoding has no Unicode signature",
            Self::InputLimit => "input exceeds codec byte limit",
            Self::OutputLimit => "output exceeds codec byte limit",
            Self::AllocationFailed => "codec output allocation failed",
        })
    }
}
impl std::error::Error for CodecError {}

/// Caller-selected byte budgets. Zero admits empty data only, never unlimited.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub input_bytes: usize,
    pub output_bytes: usize,
}

/// Complete file serialization choices. Line endings and normalization are untouched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Format {
    pub encoding: Encoding,
    pub bom: bool,
}

/// Strictly decoded text, with the observed BOM decision for subsequent encoding.
pub struct Decoded {
    pub text: String,
    pub format: Format,
}
impl fmt::Debug for Decoded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Decoded")
            .field("text_bytes", &self.text.len())
            .field("format", &self.format)
            .finish()
    }
}

/// Decodes the explicitly selected encoding. Never guesses or replaces errors.
/// Unicode encodings remove one matching leading signature; contradictory
/// signatures fail. Legacy encodings treat all bytes as data.
pub fn decode(bytes: &[u8], encoding: Encoding, limits: Limits) -> Result<Decoded, CodecError> {
    if bytes.len() > limits.input_bytes {
        return Err(CodecError::InputLimit);
    }
    let mut payload = bytes;
    let mut bom = false;
    if encoding.bom().is_some() {
        if let Some(detected) = detect_bom(bytes) {
            if detected != encoding {
                return Err(CodecError::BomMismatch);
            }
            payload = &bytes[encoding.bom().unwrap().len()..];
            bom = true;
        }
    }
    let text = conversion::decode(payload, encoding, limits.output_bytes)?;
    Ok(Decoded {
        text,
        format: Format { encoding, bom },
    })
}

/// Encodes strictly, including the requested BOM in the output budget.
/// No best-fit, replacement bytes, XML entities or Unicode normalization.
pub fn encode(text: &str, format: Format, limits: Limits) -> Result<Vec<u8>, CodecError> {
    if text.len() > limits.input_bytes {
        return Err(CodecError::InputLimit);
    }
    let mut output = Output::new(limits.output_bytes);
    if format.bom {
        output.push(format.encoding.bom().ok_or(CodecError::UnsupportedBom)?)?;
    }
    conversion::encode(text, format.encoding, &mut output)?;
    // Some legacy encoders accept characters whose decoded meaning differs
    // (for example yen/backslash mappings). Strict file saves reject that too.
    if matches!(
        format.encoding.0,
        encoding::Kind::Legacy(_) | encoding::Kind::Oem(_)
    ) {
        let roundtrip = conversion::decode(&output.bytes, format.encoding, limits.input_bytes)?;
        if roundtrip != text {
            return Err(CodecError::Unrepresentable);
        }
    }
    Ok(output.bytes)
}

struct Output {
    bytes: Vec<u8>,
    limit: usize,
}
impl Output {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }
    fn push(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        if bytes.len() > self.limit - self.bytes.len() {
            return Err(CodecError::OutputLimit);
        }
        let required = self.bytes.len() + bytes.len();
        if required > self.bytes.capacity() {
            let target = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(4096)
                .max(required)
                .min(self.limit);
            self.bytes
                .try_reserve_exact(target - self.bytes.len())
                .map_err(|_| CodecError::AllocationFailed)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn character(&mut self, value: char) -> Result<(), CodecError> {
        self.push(value.encode_utf8(&mut [0; 4]).as_bytes())
    }
}
