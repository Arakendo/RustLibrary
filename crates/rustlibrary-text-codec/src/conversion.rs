use crate::{CodecError as E, Encoding, Output, encoding::Kind, oem};

pub(crate) fn decode(bytes: &[u8], encoding: Encoding, limit: usize) -> Result<String, E> {
    let mut out = Output::new(limit);
    match encoding.0 {
        Kind::Utf8 => {
            std::str::from_utf8(bytes).map_err(|_| E::MalformedInput)?;
            out.push(bytes)?;
        }
        Kind::Ascii | Kind::Latin1 | Kind::Oem(_) => {
            for &byte in bytes {
                let value = match encoding.0 {
                    Kind::Ascii if byte > 127 => return Err(E::MalformedInput),
                    Kind::Oem(n) => oem::decode(n, byte)?,
                    _ => char::from(byte),
                };
                out.character(value)?;
            }
        }
        Kind::Utf32Le | Kind::Utf32Be => {
            if bytes.len() % 4 != 0 {
                return Err(E::MalformedInput);
            }
            for chunk in bytes.chunks_exact(4) {
                let array = [chunk[0], chunk[1], chunk[2], chunk[3]];
                let value = if encoding.0 == Kind::Utf32Le {
                    u32::from_le_bytes(array)
                } else {
                    u32::from_be_bytes(array)
                };
                out.character(char::from_u32(value).ok_or(E::MalformedInput)?)?;
            }
        }
        Kind::Utf16Le | Kind::Utf16Be => {
            if bytes.len() % 2 != 0 {
                return Err(E::MalformedInput);
            }
            let units = bytes.chunks_exact(2).map(|b| {
                if encoding.0 == Kind::Utf16Le {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            });
            for value in char::decode_utf16(units) {
                out.character(value.map_err(|_| E::MalformedInput)?)?;
            }
        }
        Kind::Legacy(backend) => {
            let mut decoder = backend.new_decoder_without_bom_handling();
            let mut consumed = 0;
            loop {
                let mut buffer = [0; 4096];
                let (result, read, written) = decoder.decode_to_utf8_without_replacement(
                    &bytes[consumed..],
                    &mut buffer,
                    true,
                );
                consumed += read;
                if matches!(result, encoding_rs::DecoderResult::Malformed(..)) {
                    return Err(E::MalformedInput);
                }
                out.push(&buffer[..written])?;
                if result == encoding_rs::DecoderResult::InputEmpty {
                    break;
                }
            }
        }
    }
    String::from_utf8(out.bytes).map_err(|_| E::MalformedInput)
}

pub(crate) fn encode(text: &str, encoding: Encoding, out: &mut Output) -> Result<(), E> {
    match encoding.0 {
        Kind::Utf8 => out.push(text.as_bytes())?,
        Kind::Ascii | Kind::Latin1 | Kind::Oem(_) => {
            for value in text.chars() {
                let byte = match encoding.0 {
                    Kind::Oem(n) => oem::encode(n, value)?,
                    Kind::Ascii if value as u32 > 127 => return Err(E::Unrepresentable),
                    _ => u8::try_from(value as u32).map_err(|_| E::Unrepresentable)?,
                };
                out.push(&[byte])?;
            }
        }
        Kind::Utf16Le | Kind::Utf16Be => {
            for unit in text.encode_utf16() {
                let bytes = if encoding.0 == Kind::Utf16Le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                out.push(&bytes)?;
            }
        }
        Kind::Utf32Le | Kind::Utf32Be => {
            for value in text.chars() {
                let bytes = if encoding.0 == Kind::Utf32Le {
                    (value as u32).to_le_bytes()
                } else {
                    (value as u32).to_be_bytes()
                };
                out.push(&bytes)?;
            }
        }
        Kind::Legacy(backend) => {
            let mut encoder = backend.new_encoder();
            let mut consumed = 0;
            loop {
                let mut buffer = [0; 4096];
                let (result, read, written) = encoder.encode_from_utf8_without_replacement(
                    &text[consumed..],
                    &mut buffer,
                    true,
                );
                consumed += read;
                if matches!(result, encoding_rs::EncoderResult::Unmappable(_)) {
                    return Err(E::Unrepresentable);
                }
                out.push(&buffer[..written])?;
                if result == encoding_rs::EncoderResult::InputEmpty {
                    break;
                }
            }
        }
    }
    Ok(())
}
