use crate::CodecError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Encoding(pub(crate) Kind);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
    Ascii,
    Latin1,
    Oem(u16),
    Legacy(&'static encoding_rs::Encoding),
}

impl Encoding {
    /// Explicit file encodings; ambiguous `ansi`, `oem`, `utf-16` are rejected.
    /// ASCII and Latin-1 retain their literal meaning, unlike browser aliases.
    pub fn for_label(label: &str) -> Result<Self, CodecError> {
        if label.len() > 40 {
            return Err(CodecError::UnsupportedEncoding);
        }
        let label = label.trim().to_ascii_lowercase();
        let kind = match label.as_str() {
            "utf-8" | "utf8" => Kind::Utf8,
            "utf-16le" => Kind::Utf16Le,
            "utf-16be" => Kind::Utf16Be,
            "utf-32le" => Kind::Utf32Le,
            "utf-32be" => Kind::Utf32Be,
            "ascii" | "us-ascii" => Kind::Ascii,
            "latin1" | "latin-1" | "iso-8859-1" => Kind::Latin1,
            _ => {
                if let Some(number) = label.strip_prefix("cp").and_then(|n| n.parse::<u16>().ok()) {
                    if OEM_PAGES.contains(&number) {
                        return Ok(Self(Kind::Oem(number)));
                    }
                    if (1250..=1258).contains(&number) {
                        return Self::for_label(&format!("windows-{number}"));
                    }
                }
                // Only canonical names (case insensitive) are admitted. In particular,
                // do not accept browser aliases that silently mean another code page.
                let backend = encoding_rs::Encoding::for_label(label.as_bytes())
                    .filter(|e| e.name().eq_ignore_ascii_case(&label))
                    .filter(|e| *e != encoding_rs::REPLACEMENT && *e != encoding_rs::X_USER_DEFINED)
                    .ok_or(CodecError::UnsupportedEncoding)?;
                Kind::Legacy(backend)
            }
        };
        Ok(Self(kind))
    }

    pub fn name(self) -> String {
        match self.0 {
            Kind::Utf8 => "UTF-8".into(),
            Kind::Utf16Le => "UTF-16LE".into(),
            Kind::Utf16Be => "UTF-16BE".into(),
            Kind::Utf32Le => "UTF-32LE".into(),
            Kind::Utf32Be => "UTF-32BE".into(),
            Kind::Ascii => "ASCII".into(),
            Kind::Latin1 => "ISO-8859-1".into(),
            Kind::Oem(n) => format!("CP{n}"),
            Kind::Legacy(e) => e.name().into(),
        }
    }

    pub(crate) fn bom(self) -> Option<&'static [u8]> {
        match self.0 {
            Kind::Utf8 => Some(b"\xEF\xBB\xBF"),
            Kind::Utf16Le => Some(b"\xFF\xFE"),
            Kind::Utf16Be => Some(b"\xFE\xFF"),
            Kind::Utf32Le => Some(b"\xFF\xFE\0\0"),
            Kind::Utf32Be => Some(b"\0\0\xFE\xFF"),
            _ => None,
        }
    }
}

pub const OEM_PAGES: &[u16] = &[
    437, 720, 737, 775, 850, 852, 855, 857, 858, 860, 861, 862, 863, 864, 865, 866, 869, 874,
];

/// Reports a leading Unicode signature only; never guesses BOM-less encodings.
/// UTF-32 signatures are checked before UTF-16 prefixes.
pub fn detect_bom(bytes: &[u8]) -> Option<Encoding> {
    [
        Kind::Utf32Le,
        Kind::Utf32Be,
        Kind::Utf8,
        Kind::Utf16Le,
        Kind::Utf16Be,
    ]
    .into_iter()
    .map(Encoding)
    .find(|e| bytes.starts_with(e.bom().unwrap()))
}
