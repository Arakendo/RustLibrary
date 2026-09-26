use rustlibrary_text_codec::{Encoding, Limits, decode, encode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let limits = Limits {
        input_bytes: 1024,
        output_bytes: 4096,
    };
    let decoded = decode(b"caf\xe9\r\n", Encoding::for_label("cp1252")?, limits)?;
    assert_eq!(decoded.text, "café\r\n");
    assert_eq!(
        encode(&decoded.text, decoded.format, limits)?,
        b"caf\xe9\r\n"
    );
    println!("Windows-1252 text round-trip succeeded");
    Ok(())
}
