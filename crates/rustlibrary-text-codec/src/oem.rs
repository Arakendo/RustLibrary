use crate::CodecError;

macro_rules! dispatch {
    ($number:expr, $value:expr, $convert:ident, $( $n:literal => $t:ty ),* ) => {
        match $number { $( $n => $convert::<$t>($value), )* _ => Err(CodecError::UnsupportedEncoding) }
    };
}

fn decode_one<T: TryFrom<u8> + Into<char>>(byte: u8) -> Result<char, CodecError> {
    T::try_from(byte)
        .map(Into::into)
        .map_err(|_| CodecError::MalformedInput)
}
fn encode_one<T: TryFrom<char> + Into<u8>>(value: char) -> Result<u8, CodecError> {
    T::try_from(value)
        .map(Into::into)
        .map_err(|_| CodecError::Unrepresentable)
}

pub(crate) fn decode(number: u16, byte: u8) -> Result<char, CodecError> {
    use oem_cp::*;
    dispatch!(number, byte, decode_one, 437=>Cp437,720=>Cp720,737=>Cp737,775=>Cp775,
        850=>Cp850,852=>Cp852,855=>Cp855,857=>Cp857,858=>Cp858,860=>Cp860,
        861=>Cp861,862=>Cp862,863=>Cp863,864=>Cp864,865=>Cp865,866=>Cp866,869=>Cp869,874=>Cp874)
}
pub(crate) fn encode(number: u16, value: char) -> Result<u8, CodecError> {
    use oem_cp::*;
    dispatch!(number, value, encode_one, 437=>Cp437,720=>Cp720,737=>Cp737,775=>Cp775,
        850=>Cp850,852=>Cp852,855=>Cp855,857=>Cp857,858=>Cp858,860=>Cp860,
        861=>Cp861,862=>Cp862,863=>Cp863,864=>Cp864,865=>Cp865,866=>Cp866,869=>Cp869,874=>Cp874)
}
