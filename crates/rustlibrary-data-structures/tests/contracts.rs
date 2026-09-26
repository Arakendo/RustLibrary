use rustlibrary_data_structures::{BufferError, GapBuffer, TextRope};

#[test]
fn gap_matches_vector_through_moves_growth_and_deletions() {
    let mut gap = GapBuffer::new(2048);
    let mut model = Vec::new();
    let mut seed = 17_u64;
    for step in 0..4000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let at = (seed as usize) % (model.len() + 1);
        gap.move_gap(at).unwrap();
        match step % 4 {
            0 if at > 0 => {
                assert_eq!(gap.delete_before(), Some(model.remove(at - 1)));
            }
            1 if at < model.len() => {
                assert_eq!(gap.delete_after(), Some(model.remove(at)));
            }
            _ => {
                gap.insert(step).unwrap();
                model.insert(at, step);
            }
        }
        assert_eq!(gap.iter().copied().collect::<Vec<_>>(), model);
        for (i, value) in model.iter().enumerate() {
            assert_eq!(gap.get(i), Some(value));
        }
        assert_eq!(gap.get(model.len()), None);
    }
}

#[test]
fn gap_returns_ownership_and_drops_each_value_once() {
    use std::{cell::Cell, rc::Rc};
    #[derive(Debug)]
    struct Owned(Rc<Cell<usize>>);
    impl Drop for Owned {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let mut gap = GapBuffer::new(2);
    gap.insert(Owned(drops.clone())).unwrap();
    gap.insert(Owned(drops.clone())).unwrap();
    gap.move_gap(0).unwrap();
    let (error, value) = gap.insert(Owned(drops.clone())).unwrap_err();
    assert_eq!(error, BufferError::LimitExceeded);
    assert_eq!(drops.get(), 0);
    drop(value);
    drop(gap.delete_after());
    assert_eq!(gap.move_gap(2), Err(BufferError::OutOfBounds));
    drop(gap);
    assert_eq!(drops.get(), 3);
    assert_eq!(
        GapBuffer::new(0).insert(42),
        Err((BufferError::LimitExceeded, 42))
    );
}

#[test]
fn rope_matches_string_for_unicode_edits_and_preserves_clone() {
    let mut model = "a😀\r\néאב".repeat(1200);
    let mut rope = TextRope::new(&model, 100_000).unwrap();
    let snapshot = rope.clone();
    let original = model.clone();
    let mut seed = 42_u64;
    for step in 0..500 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let positions: Vec<_> = model
            .char_indices()
            .map(|(i, _)| i)
            .chain([model.len()])
            .collect();
        let a = seed as usize % positions.len();
        let b = (a + step % 4).min(positions.len() - 1);
        let range = positions[a]..positions[b];
        let text = ["", "新", "😀", "e\u{301}", "\r\n"][step % 5];
        model.replace_range(range.clone(), text);
        rope.replace(range, text).unwrap();
        assert_eq!(rope.to_text(), model);
    }
    assert_eq!(snapshot.to_text(), original);
    assert_eq!(rope.chunks().collect::<String>(), model);
}

#[test]
fn rope_rejects_invalid_edits_and_positions_without_mutation() {
    let mut rope = TextRope::new("a😀b", 6).unwrap();
    assert_eq!(rope.replace(2..3, ""), Err(BufferError::InvalidBoundary));
    assert_eq!(rope.replace(6..7, ""), Err(BufferError::OutOfBounds));
    assert_eq!(rope.replace(0..0, "x"), Err(BufferError::LimitExceeded));
    assert_eq!(rope.to_text(), "a😀b");
    assert_eq!(rope.byte_to_utf16(5), Ok(3));
    assert_eq!(rope.utf16_to_byte(3), Ok(5));
    assert_eq!(rope.utf16_to_byte(2), Err(BufferError::InvalidBoundary));
    assert_eq!(rope.byte_to_utf16(2), Err(BufferError::InvalidBoundary));
    assert_eq!(rope.utf16_to_byte(5), Err(BufferError::OutOfBounds));
    assert_eq!(rope.utf16_to_byte(4), Ok(6));
}

#[test]
fn line_contract_is_cr_lf_crlf_and_debug_does_not_leak_content() {
    let rope = TextRope::new("a\r\nb\rc\nd\u{2028}secret", 100).unwrap();
    assert_eq!(rope.line_count(), 4);
    assert_eq!(
        (0..4)
            .map(|n| rope.line_start_byte(n).unwrap())
            .collect::<Vec<_>>(),
        vec![0, 3, 5, 7]
    );
    assert_eq!(rope.line_start_byte(4), Err(BufferError::OutOfBounds));
    assert!(!format!("{rope:?}").contains("secret"));
    assert_eq!(TextRope::new("", 0).unwrap().line_count(), 1);
    assert!(matches!(
        TextRope::new("x", 0),
        Err(BufferError::LimitExceeded)
    ));
}
