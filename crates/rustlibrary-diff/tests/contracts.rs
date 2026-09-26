use rustlibrary_diff::{DiffError as E, Limits, diff_lines};

fn reconstruct(old: &str, new: &str) {
    let changes = diff_lines(old, new, Limits::default()).unwrap();
    let mut rebuilt = String::new();
    let (mut old_at, mut new_at) = (0, 0);
    for change in &changes {
        assert_eq!(
            &old[old_at..change.old_bytes.start],
            &new[new_at..change.new_bytes.start]
        );
        rebuilt.push_str(&old[old_at..change.old_bytes.start]);
        rebuilt.push_str(&new[change.new_bytes.clone()]);
        old_at = change.old_bytes.end;
        new_at = change.new_bytes.end;
    }
    assert_eq!(&old[old_at..], &new[new_at..]);
    rebuilt.push_str(&old[old_at..]);
    assert_eq!(rebuilt, new);
    assert_eq!(changes.is_empty(), old == new);
}

#[test]
fn source_roundtrips_cover_empty_unicode_endings_and_repeated_lines() {
    let cases = [
        "",
        "a",
        "a\n",
        "a\r\n",
        "\r\n",
        "a\rb\n",
        "😀\ne\u{301}\nאב",
        "a\nb\na\n",
        "a\na\nb\n",
        "--- header\n+++ payload\n",
    ];
    for old in cases {
        for new in cases {
            reconstruct(old, new);
        }
    }
}

#[test]
fn deterministic_generated_documents_reconstruct_exactly() {
    let mut seed = 47_u64;
    for _ in 0..200 {
        let mut texts = [String::new(), String::new()];
        for text in &mut texts {
            for _ in 0..20 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                text.push_str(["a\n", "b\r\n", "😀\r", "\n", ""][seed as usize % 5]);
            }
        }
        reconstruct(&texts[0], &texts[1]);
    }
}

#[test]
fn insertion_deletion_replacement_and_eof_ranges_are_explicit() {
    let insert = diff_lines("a\n", "a\nb\n", Limits::default()).unwrap();
    assert_eq!(insert[0].old_lines, 1..1);
    assert_eq!(insert[0].new_lines, 1..2);
    assert_eq!(insert[0].old_bytes, 2..2);
    assert_eq!(insert[0].new_bytes, 2..4);
    let delete = diff_lines("a\nb\n", "b\n", Limits::default()).unwrap();
    assert_eq!(delete[0].old_lines, 0..1);
    assert_eq!(delete[0].new_lines, 0..0);
    let replace = diff_lines("a\n", "b\n", Limits::default()).unwrap();
    assert_eq!(replace[0].old_lines, 0..1);
    assert_eq!(replace[0].new_lines, 0..1);
    let ending = diff_lines("a\n", "a", Limits::default()).unwrap();
    assert_eq!(ending.len(), 1);
}

#[test]
fn each_budget_failure_is_explicit_not_clean_or_partial() {
    let limits = Limits::default();
    assert_eq!(
        diff_lines(
            "a",
            "b",
            Limits {
                input_bytes: 1,
                ..limits
            }
        ),
        Err(E::InputLimit)
    );
    assert_eq!(
        diff_lines("a", "b", Limits { lines: 1, ..limits }),
        Err(E::LineLimit)
    );
    assert_eq!(
        diff_lines("a", "b", Limits { cells: 3, ..limits }),
        Err(E::CellLimit)
    );
    assert_eq!(
        diff_lines("a", "b", Limits { work: 6, ..limits }),
        Err(E::WorkLimit)
    );
    assert_eq!(
        diff_lines(
            "a\nx\nb",
            "c\nx\nd",
            Limits {
                changes: 1,
                ..limits
            }
        ),
        Err(E::ChangeLimit)
    );
    assert!(
        diff_lines(
            "",
            "",
            Limits {
                input_bytes: 0,
                lines: 0,
                changes: 0,
                ..limits
            }
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        diff_lines("", "", Limits { cells: 0, ..limits }),
        Err(E::CellLimit)
    );
}

#[test]
fn long_line_comparison_consumes_work_and_errors_do_not_expose_text() {
    let limits = Limits {
        work: 250,
        ..Limits::default()
    };
    assert_eq!(
        diff_lines(&"a".repeat(100), &"a".repeat(100), limits),
        Err(E::WorkLimit)
    );
    assert!(!format!("{:?}", diff_lines("secret", "new", Limits::default())).contains("secret"));
}

#[test]
fn repeated_lines_keep_common_subsequence_with_deterministic_ties() {
    let changes = diff_lines("a\nb\na\n", "b\na\nb\n", Limits::default()).unwrap();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].old_lines, 0..1);
    assert_eq!(changes[0].new_lines, 0..0);
    assert_eq!(changes[1].old_lines, 3..3);
    assert_eq!(changes[1].new_lines, 2..3);
}
