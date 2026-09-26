use rustlibrary_data_structures::{BufferError, TextRope};

// Independent byte scanner: explicit CR/LF semantics, without Ropey indexing.
fn model(text: &str) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    let (mut byte, mut line, mut column) = (0, 0, 0);
    out.push((byte, line, column));
    while byte < text.len() {
        let ch = text[byte..].chars().next().unwrap();
        if ch == '\r' || ch == '\n' {
            byte += if text[byte..].starts_with("\r\n") {
                2
            } else {
                1
            };
            line += 1;
            column = 0;
        } else {
            byte += ch.len_utf8();
            column += ch.len_utf16();
        }
        out.push((byte, line, column));
    }
    out
}

#[test]
fn positions_match_independent_model_across_chunks_and_unicode() {
    for text in [
        String::new(),
        "\r\n\r\n".into(),
        "a😀e\u{301}\tאב\r\n新\rnext\n\u{2028}\u{85}".repeat(1200),
    ] {
        let rope = TextRope::new(&text, 100_000).unwrap();
        for (byte, line, column) in model(&text) {
            assert_eq!(rope.byte_to_line_utf16(byte), Ok((line, column)));
            assert_eq!(rope.line_utf16_to_byte(line, column), Ok(byte));
        }
    }
}

#[test]
fn rejects_crlf_interior_surrogates_overflow_and_columns_in_next_line() {
    let rope = TextRope::new("😀\r\nx\n", 100).unwrap();
    assert_eq!(rope.byte_to_line_utf16(4), Ok((0, 2)));
    assert_eq!(
        rope.byte_to_line_utf16(5),
        Err(BufferError::InvalidBoundary)
    );
    assert_eq!(
        rope.byte_to_line_utf16(1),
        Err(BufferError::InvalidBoundary)
    );
    assert_eq!(
        rope.line_utf16_to_byte(0, 1),
        Err(BufferError::InvalidBoundary)
    );
    assert_eq!(rope.line_utf16_to_byte(0, 3), Err(BufferError::OutOfBounds));
    assert_eq!(rope.line_utf16_to_byte(1, 2), Err(BufferError::OutOfBounds));
    assert_eq!(rope.line_utf16_to_byte(2, 0), Ok(8));
    assert_eq!(rope.line_utf16_to_byte(2, 1), Err(BufferError::OutOfBounds));
    assert_eq!(
        rope.line_utf16_to_byte(usize::MAX, 0),
        Err(BufferError::OutOfBounds)
    );
    assert_eq!(
        rope.line_utf16_to_byte(0, usize::MAX),
        Err(BufferError::OutOfBounds)
    );
    assert_eq!(
        rope.byte_to_line_utf16(usize::MAX),
        Err(BufferError::OutOfBounds)
    );
}

#[test]
fn edit_reindexes_lines_without_changing_retained_snapshot_positions() {
    let mut rope = TextRope::new("a\r\nb", 100).unwrap();
    let before = rope.clone();
    rope.replace(1..3, "😀\r").unwrap();
    assert_eq!(rope.line_utf16_to_byte(1, 0), Ok(6));
    assert_eq!(rope.byte_to_line_utf16(5), Ok((0, 3)));
    assert_eq!(before.line_utf16_to_byte(1, 0), Ok(3));
}
