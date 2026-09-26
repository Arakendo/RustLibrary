use rustlibrary_text_codec::{
    CodecError as E, Encoding, Format, Limits, OEM_PAGES, decode, detect_bom, encode,
};
const LIMITS: Limits = Limits {
    input_bytes: 100_000,
    output_bytes: 100_000,
};
fn enc(label: &str) -> Encoding {
    Encoding::for_label(label).unwrap()
}
fn format(label: &str) -> Format {
    Format {
        encoding: enc(label),
        bom: false,
    }
}

#[test]
fn windows_and_literal_latin1_are_distinct() {
    assert_eq!(
        decode(b"\x80\x93caf\xe9\x94\r\n", enc("windows-1252"), LIMITS)
            .unwrap()
            .text,
        "€“café”\r\n"
    );
    assert_eq!(
        decode(b"\x80", enc("latin1"), LIMITS).unwrap().text,
        "\u{80}"
    );
    assert_eq!(encode("\u{80}", format("latin1"), LIMITS).unwrap(), [0x80]);
    assert_eq!(
        decode(b"\x80", enc("ascii"), LIMITS).unwrap_err(),
        E::MalformedInput
    );
    assert_eq!(
        encode("é", format("ascii"), LIMITS),
        Err(E::Unrepresentable)
    );
    assert_eq!(
        encode("€", format("latin1"), LIMITS),
        Err(E::Unrepresentable)
    );
}

#[test]
fn windows_codepages_roundtrip_representative_scripts() {
    for (label, text) in [
        ("cp1250", "Zażółć"),
        ("cp1251", "Привет"),
        ("cp1252", "café €"),
        ("cp1253", "Ελλάδα"),
        ("cp1254", "İstanbul"),
        ("cp1255", "שלום"),
        ("cp1256", "مرحبا"),
        ("cp1257", "Rīga"),
        ("cp1258", "abc"),
    ] {
        let bytes = encode(text, format(label), LIMITS).unwrap();
        assert_eq!(decode(&bytes, enc(label), LIMITS).unwrap().text, text);
    }
}

#[test]
fn oem_pages_are_explicit_and_preserve_controls() {
    assert_eq!(
        decode(&[0x82, 0xb3, 0xc4], enc("cp437"), LIMITS)
            .unwrap()
            .text,
        "é│─"
    );
    assert_eq!(decode(&[0x9b], enc("cp850"), LIMITS).unwrap().text, "ø");
    assert_eq!(decode(&[0x9b], enc("cp437"), LIMITS).unwrap().text, "¢");
    for page in OEM_PAGES {
        let encoding = enc(&format!("cp{page}"));
        for byte in 0..=255 {
            if let Ok(decoded) = decode(&[byte], encoding, LIMITS) {
                let output = encode(&decoded.text, decoded.format, LIMITS).unwrap();
                assert_eq!(
                    decode(&output, encoding, LIMITS).unwrap().text,
                    decoded.text
                );
            }
        }
    }
    assert_eq!(
        decode(&[0, 0x1a, 13, 10], enc("cp437"), LIMITS)
            .unwrap()
            .text,
        "\0\u{1a}\r\n"
    );
}

#[test]
fn unicode_bom_and_newline_roundtrips() {
    let text = "a\r\nb\nc\r😀e\u{301}שלום\0";
    for label in ["utf-8", "utf-16le", "utf-16be", "utf-32le", "utf-32be"] {
        for bom in [false, true] {
            let format = Format {
                encoding: enc(label),
                bom,
            };
            let bytes = encode(text, format, LIMITS).unwrap();
            let decoded = decode(&bytes, format.encoding, LIMITS).unwrap();
            assert_eq!(decoded.text, text);
            assert_eq!(decoded.format, format);
            assert_eq!(
                encode(&decoded.text, decoded.format, LIMITS).unwrap(),
                bytes
            );
        }
    }
    assert_eq!(
        encode(
            "A",
            Format {
                encoding: enc("utf-16le"),
                bom: true
            },
            LIMITS
        )
        .unwrap(),
        [0xff, 0xfe, 65, 0]
    );
    assert_eq!(detect_bom(b"\xff\xfe\0\0"), Some(enc("utf-32le")));
    assert!(detect_bom(b"plain ASCII").is_none());
}

#[test]
fn malformed_inputs_and_bom_mismatches_fail() {
    for (label, bytes) in [
        ("utf-8", &b"\xc0\xaf"[..]),
        ("utf-16le", &b"\0\xd8"[..]),
        ("utf-16le", &b"A"[..]),
        ("utf-32le", &b"\0\xd8\0\0"[..]),
        ("utf-32le", &b"\0\0\x11\0"[..]),
        ("Shift_JIS", &b"\x82"[..]),
    ] {
        assert_eq!(
            decode(bytes, enc(label), LIMITS).unwrap_err(),
            E::MalformedInput
        );
    }
    assert_eq!(
        decode(b"\xff\xfeA\0", enc("utf-8"), LIMITS).unwrap_err(),
        E::BomMismatch
    );
    assert_eq!(
        encode(
            "A",
            Format {
                encoding: enc("cp1252"),
                bom: true
            },
            LIMITS
        ),
        Err(E::UnsupportedBom)
    );
    assert!(decode(b"\xff\xfe", enc("cp1252"), LIMITS).is_ok());
}

#[test]
fn multibyte_and_stateful_encodings_finalize_and_roundtrip() {
    for (label, text) in [
        ("Shift_JIS", "日本語"),
        ("EUC-JP", "日本語"),
        ("ISO-2022-JP", "日本語"),
        ("GBK", "中文"),
        ("gb18030", "中文😀"),
        ("Big5", "中文"),
        ("EUC-KR", "한국어"),
        ("KOI8-R", "Привет"),
    ] {
        let text = text.repeat(3000);
        let bytes = encode(&text, format(label), LIMITS).unwrap();
        assert_eq!(decode(&bytes, enc(label), LIMITS).unwrap().text, text);
    }
    assert_eq!(
        encode("😀", format("windows-1252"), LIMITS),
        Err(E::Unrepresentable)
    );
    assert_eq!(
        encode("¥", format("Shift_JIS"), LIMITS),
        Err(E::Unrepresentable)
    );
}

#[test]
fn finite_limits_include_expansion_and_bom() {
    assert_eq!(
        decode(
            b"ab",
            enc("utf-8"),
            Limits {
                input_bytes: 1,
                output_bytes: 10
            }
        )
        .unwrap_err(),
        E::InputLimit
    );
    assert_eq!(
        decode(
            &[0x80],
            enc("windows-1252"),
            Limits {
                input_bytes: 1,
                output_bytes: 2
            }
        )
        .unwrap_err(),
        E::OutputLimit
    );
    assert_eq!(
        encode(
            "a",
            format("utf-16le"),
            Limits {
                input_bytes: 1,
                output_bytes: 1
            }
        ),
        Err(E::OutputLimit)
    );
    assert_eq!(
        encode(
            "",
            Format {
                encoding: enc("utf-8"),
                bom: true
            },
            Limits {
                input_bytes: 0,
                output_bytes: 2
            }
        ),
        Err(E::OutputLimit)
    );
    assert!(
        encode(
            "",
            format("utf-8"),
            Limits {
                input_bytes: 0,
                output_bytes: 0
            }
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        decode(
            &[0x80],
            enc("windows-1252"),
            Limits {
                input_bytes: 1,
                output_bytes: 3
            }
        )
        .unwrap()
        .text,
        "€"
    );
}

#[test]
fn ambiguous_and_unsupported_labels_are_not_guessed() {
    for label in [
        "ansi",
        "oem",
        "utf-16",
        "utf-32",
        "utf-7",
        "replacement",
        "x-user-defined",
        "cp999",
        "iso-8859-8-i-wrong",
    ] {
        assert_eq!(Encoding::for_label(label), Err(E::UnsupportedEncoding));
    }
}
