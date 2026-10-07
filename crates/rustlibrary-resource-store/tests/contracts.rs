use rustlibrary_resource_store::{
    Attributes, CasePolicy, EntryKind, Error, Limits, ListOptions, ResourceStore, Snapshot,
    WriteMode,
};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

fn all(view: &Snapshot) -> Vec<rustlibrary_resource_store::Entry> {
    view.list(
        "",
        ListOptions {
            recursive: true,
            include_hidden: true,
        },
    )
    .unwrap()
}

fn unchanged(store: &ResourceStore, before: &Snapshot) {
    assert_eq!(store.generation(), before.generation());
    assert_eq!(all(store), all(before));
    assert_eq!(store.total_bytes(), before.total_bytes());
    for entry in all(before) {
        if entry.kind == EntryKind::File {
            assert_eq!(
                store.read(&entry.path).unwrap(),
                before.read(&entry.path).unwrap()
            );
        }
    }
}

#[test]
fn document_bundle_supports_navigation_and_sibling_resolution() {
    let mut s = ResourceStore::default();
    s.create_directory("drafts").unwrap();
    s.write_many(
        [
            ("data.xml", b"<data/>".as_slice()),
            ("transform.xsl", b"<stylesheet/>".as_slice()),
            ("common/utilities.xsl", b"<helper/>".as_slice()),
            ("assets/logo.png", b"\x89PNG".as_slice()),
        ],
        WriteMode::Create,
    )
    .unwrap();
    assert_eq!(s.len(), 4);
    assert_eq!(s.entry("drafts").unwrap().kind, EntryKind::Directory);
    assert!(s.list("drafts", ListOptions::default()).unwrap().is_empty());
    let root = s.list("", ListOptions::default()).unwrap();
    assert_eq!(
        root.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
        ["assets", "common", "data.xml", "drafts", "transform.xsl"]
    );
    let related = s.resolve("common/utilities.xsl", "../data.xml").unwrap();
    assert_eq!(s.read_text(&related).unwrap(), "<data/>");
    assert_eq!(
        s.resolve("data.xml", "./common/utilities.xsl").unwrap(),
        "common/utilities.xsl"
    );
}

#[test]
fn invalid_addresses_fail_without_creating_parents() {
    let mut s = ResourceStore::default();
    for path in [
        "/x",
        "x/",
        "a//b",
        ".",
        "..",
        "a/../b",
        "C:/x",
        "mem:/x",
        r"a\b",
        "a?query",
        "a#fragment",
        "%2e%2e/x",
        "a\0b",
        "a\nb",
    ] {
        let before = s.snapshot();
        assert_eq!(
            s.write(path, b"x", WriteMode::Upsert),
            Err(Error::InvalidPath),
            "{path:?}"
        );
        unchanged(&s, &before);
    }
    assert_eq!(
        s.write("", b"x", WriteMode::Create),
        Err(Error::IsDirectory)
    );
    assert_eq!(s.entry("").unwrap().kind, EntryKind::Directory);
}

#[test]
fn relative_resolution_rejects_escape_and_disguised_host_paths() {
    let s = ResourceStore::default();
    for reference in [
        "../../x",
        "/x",
        "C:/../x",
        "http:/../x",
        "%ab/../x",
        r"a\b/../x",
        "a//x",
        "",
    ] {
        assert_eq!(
            s.resolve("dir/a", reference),
            Err(Error::InvalidPath),
            "{reference}"
        );
    }
    assert_eq!(s.resolve("a", "../x"), Err(Error::InvalidPath));
    assert_eq!(s.resolve("a/b/c", "../d"), Ok("a/d".into()));
}

#[test]
fn case_policy_is_explicit_and_unicode_is_preserved() {
    let mut sensitive = ResourceStore::default();
    sensitive.write("A", [], WriteMode::Create).unwrap();
    sensitive.write("a", [], WriteMode::Create).unwrap();
    assert_eq!(sensitive.len(), 2);
    let mut s = ResourceStore::new(CasePolicy::AsciiInsensitive, Limits::default());
    let e = s
        .write_text("Dir/A.TXT", "hello", WriteMode::Create)
        .unwrap();
    assert_eq!(e.path, "dir/a.txt");
    assert_eq!(s.read_text("DIR/a.Txt").unwrap(), "hello");
    assert_eq!(
        s.write("dir/A.txt", [], WriteMode::Create),
        Err(Error::AlreadyExists)
    );
    s.write_text("資料/É", "🌍", WriteMode::Create).unwrap();
    s.write_text("資料/é", "different", WriteMode::Create)
        .unwrap();
    assert_eq!(s.read_text("資料/É").unwrap(), "🌍");
}

#[test]
fn explicit_write_modes_and_versions_prevent_lost_updates_and_recreation_aba() {
    let mut s = ResourceStore::default();
    assert_eq!(s.write("a", [], WriteMode::Replace), Err(Error::NotFound));
    let first = s.write("a", b"1", WriteMode::Create).unwrap();
    assert_eq!(
        s.write("a", [], WriteMode::Create),
        Err(Error::AlreadyExists)
    );
    let second = s
        .write("a", b"2", WriteMode::CompareVersion(first.version))
        .unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(
        s.write("a", [], WriteMode::CompareVersion(first.version)),
        Err(Error::VersionConflict {
            expected: first.version,
            actual: second.version
        })
    );
    s.remove("a", false).unwrap();
    let third = s.write("a", b"3", WriteMode::Create).unwrap();
    assert_ne!(first.id, third.id);
    assert!(third.version > second.version);
    assert!(matches!(
        s.write("a", [], WriteMode::CompareVersion(second.version)),
        Err(Error::VersionConflict { .. })
    ));
}

#[test]
fn bytes_streams_and_snapshots_survive_mutation_and_drop() {
    let mut input = b"first".to_vec();
    let mut s = ResourceStore::default();
    s.write("a", &input, WriteMode::Create).unwrap();
    input[0] = b'X';
    let snapshot = s.snapshot();
    let bytes = s.read("a").unwrap();
    let mut stream = s.open("a").unwrap();
    s.write("a", b"second", WriteMode::Replace).unwrap();
    s.clear().unwrap();
    drop(s);
    assert_eq!(&*bytes, b"first");
    assert_eq!(snapshot.read_text("a").unwrap(), "first");
    stream.seek(SeekFrom::Start(1)).unwrap();
    let mut tail = String::new();
    stream.read_to_string(&mut tail).unwrap();
    assert_eq!(tail, "irst");
}

#[test]
fn hidden_and_read_only_are_inherited_but_hidden_does_not_deny_reads() {
    let mut s = ResourceStore::default();
    s.write("dir/a", b"data", WriteMode::Create).unwrap();
    s.set_attributes(
        "dir",
        Attributes {
            hidden: true,
            read_only: true,
        },
    )
    .unwrap();
    assert!(
        s.list(
            "",
            ListOptions {
                recursive: true,
                include_hidden: false
            }
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(all(&s).len(), 2);
    assert_eq!(s.read_text("dir/a").unwrap(), "data");
    let e = s.entry("dir/a").unwrap();
    assert!(!e.attributes.hidden && e.effective_attributes.hidden);
    let before = s.snapshot();
    assert_eq!(
        s.write("dir/b", [], WriteMode::Create),
        Err(Error::ReadOnly)
    );
    assert_eq!(
        s.set_attributes("dir/a", Attributes::default()),
        Err(Error::ReadOnly)
    );
    assert_eq!(s.move_entry("dir/a", "elsewhere"), Err(Error::ReadOnly));
    unchanged(&s, &before);
    s.set_attributes("dir", Attributes::default()).unwrap();
    s.write("dir/b", [], WriteMode::Create).unwrap();
}

#[test]
fn read_only_descendant_makes_recursive_operations_atomic() {
    let mut s = ResourceStore::default();
    s.write_many(
        [("tree/a", b"a".as_slice()), ("tree/z", b"z".as_slice())],
        WriteMode::Create,
    )
    .unwrap();
    s.set_attributes(
        "tree/z",
        Attributes {
            hidden: false,
            read_only: true,
        },
    )
    .unwrap();
    let before = s.snapshot();
    assert_eq!(s.remove("tree", true), Err(Error::ReadOnly));
    assert_eq!(
        s.move_entry("tree", "new/parent/tree"),
        Err(Error::ReadOnly)
    );
    assert_eq!(s.clear(), Err(Error::ReadOnly));
    unchanged(&s, &before);
    s.copy("tree", "copied").unwrap();
    assert!(s.entry("copied/z").unwrap().attributes.read_only);
}

#[test]
fn root_attributes_are_inherited_and_owner_can_clear_them() {
    let mut s = ResourceStore::default();
    s.set_attributes(
        "",
        Attributes {
            hidden: true,
            read_only: true,
        },
    )
    .unwrap();
    assert_eq!(s.create_directory("a"), Err(Error::ReadOnly));
    assert_eq!(s.write("b", [], WriteMode::Create), Err(Error::ReadOnly));
    assert_eq!(s.remove("", true), Err(Error::InvalidPath));
    s.set_attributes("", Attributes::default()).unwrap();
    s.create_directory("a").unwrap();
    s.clear().unwrap();
    assert!(s.contains("").unwrap());
}

#[test]
fn file_directory_collisions_and_missing_entries_have_typed_errors() {
    let mut s = ResourceStore::default();
    s.write("file", [], WriteMode::Create).unwrap();
    let before = s.snapshot();
    assert_eq!(s.create_directory("file"), Err(Error::AlreadyExists));
    assert_eq!(
        s.write("file/deep/child", [], WriteMode::Create),
        Err(Error::NotDirectory)
    );
    assert_eq!(
        s.list("file", ListOptions::default()),
        Err(Error::NotDirectory)
    );
    assert_eq!(s.read(""), Err(Error::IsDirectory));
    assert_eq!(s.entry("absent"), Err(Error::NotFound));
    assert!(!s.remove("absent", false).unwrap());
    unchanged(&s, &before);
    s.create_directory("empty").unwrap();
    assert_eq!(
        s.write("empty", [], WriteMode::Upsert),
        Err(Error::IsDirectory)
    );
    let gen_before = s.generation();
    s.create_directory("empty").unwrap();
    assert_eq!(s.generation(), gen_before);
}

#[test]
fn copy_and_move_preserve_tree_and_distinguish_identity_from_content() {
    let mut s = ResourceStore::default();
    s.create_directory("tree/empty").unwrap();
    let original = s
        .write_text("tree/file", "payload", WriteMode::Create)
        .unwrap();
    s.write("tree-other", [], WriteMode::Create).unwrap();
    s.copy("tree", "copy").unwrap();
    assert!(Arc::ptr_eq(
        &s.read("tree/file").unwrap(),
        &s.read("copy/file").unwrap()
    ));
    assert_ne!(s.entry("copy/file").unwrap().id, original.id);
    assert!(s.content_equals("tree/file", "copy/file").unwrap());
    s.move_entry("tree", "nested/moved").unwrap();
    assert_eq!(
        s.entry_by_id(original.id).unwrap().path,
        "nested/moved/file"
    );
    assert!(s.contains("nested/moved/empty").unwrap());
    assert!(!s.contains("tree").unwrap());
    assert!(s.contains("tree-other").unwrap());
    assert_eq!(s.total_bytes(), 14);
    s.remove("nested/moved", true).unwrap();
    assert_eq!(s.entry_by_id(original.id), Err(Error::NotFound));
    assert_eq!(s.total_bytes(), 7);
}

#[test]
fn invalid_transfers_do_not_mutate_state() {
    let mut s = ResourceStore::default();
    s.write("a/x", [], WriteMode::Create).unwrap();
    s.create_directory("b").unwrap();
    let before = s.snapshot();
    for (src, dst, error) in [
        ("a", "a", Error::InvalidPath),
        ("a", "a/nested", Error::InvalidPath),
        ("a", "b", Error::AlreadyExists),
        ("missing", "new", Error::NotFound),
        ("", "new", Error::InvalidPath),
        ("a", "", Error::InvalidPath),
        ("a", "a/x/child", Error::InvalidPath),
    ] {
        assert_eq!(s.copy(src, dst), Err(error.clone()));
        assert_eq!(s.move_entry(src, dst), Err(error));
        unchanged(&s, &before);
    }
}

#[test]
fn nonrecursive_delete_requires_empty_folder_and_matching_segment() {
    let mut s = ResourceStore::default();
    s.write("a/file", [], WriteMode::Create).unwrap();
    s.write("ab/file", [], WriteMode::Create).unwrap();
    assert_eq!(s.remove("a", false), Err(Error::DirectoryNotEmpty));
    assert!(s.remove("a", true).unwrap());
    assert!(s.contains("ab/file").unwrap());
    assert!(s.remove("ab/file", false).unwrap());
    assert!(s.contains("ab").unwrap());
    assert!(s.remove("ab", false).unwrap());
}

#[test]
fn batch_collision_or_invalid_input_rolls_back_prior_writes_and_metadata() {
    let mut s = ResourceStore::default();
    s.write_text("existing", "old", WriteMode::Create).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.write_many(
            [
                ("new/a", b"new".as_slice()),
                ("existing", b"changed".as_slice())
            ],
            WriteMode::Create
        ),
        Err(Error::AlreadyExists)
    );
    unchanged(&s, &before);
    assert_eq!(
        s.write_many(
            [
                ("existing", b"new".as_slice()),
                ("bad/../x", b"x".as_slice())
            ],
            WriteMode::Upsert
        ),
        Err(Error::InvalidPath)
    );
    unchanged(&s, &before);
    assert_eq!(
        s.write_many(
            [("dup", b"1".as_slice()), ("dup", b"2".as_slice())],
            WriteMode::Create
        ),
        Err(Error::AlreadyExists)
    );
    unchanged(&s, &before);
}

#[test]
fn retention_limits_include_parents_and_logical_copies() {
    let limits = Limits {
        max_entries: 3,
        max_total_bytes: 4,
        max_resource_bytes: 3,
        ..Limits::default()
    };
    let mut s = ResourceStore::new(CasePolicy::Sensitive, limits);
    s.write("a/x", b"abc", WriteMode::Create).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.write("b/c", [], WriteMode::Create),
        Err(Error::LimitExceeded("entries"))
    );
    assert_eq!(s.copy("a", "b"), Err(Error::LimitExceeded("entries")));
    assert_eq!(s.copy("a/x", "y"), Err(Error::LimitExceeded("total bytes")));
    assert_eq!(
        s.write("y", b"abcd", WriteMode::Create),
        Err(Error::LimitExceeded("resource bytes"))
    );
    unchanged(&s, &before);
    s.write("a/x", b"a", WriteMode::Replace).unwrap();
    s.write("y", b"bcd", WriteMode::Create).unwrap();
    assert_eq!(s.total_bytes(), 4);
    let before = s.snapshot();
    assert_eq!(
        s.append_text("a/x", "!"),
        Err(Error::LimitExceeded("total bytes"))
    );
    unchanged(&s, &before);
}

#[test]
fn zero_limits_allow_only_root_and_failed_batches_respect_budget() {
    let limits = Limits {
        max_entries: 0,
        max_total_bytes: 0,
        max_resource_bytes: 0,
        max_path_bytes: 0,
        max_depth: 0,
    };
    let mut s = ResourceStore::new(CasePolicy::Sensitive, limits);
    assert!(s.create_directory("").is_ok());
    assert_eq!(
        s.create_directory("x"),
        Err(Error::LimitExceeded("path bytes"))
    );
    assert!(s.is_empty());
    let mut s = ResourceStore::new(
        CasePolicy::Sensitive,
        Limits {
            max_total_bytes: 3,
            ..Limits::default()
        },
    );
    let before = s.snapshot();
    assert_eq!(
        s.write_many(
            [("a", b"aa".as_slice()), ("b", b"bb".as_slice())],
            WriteMode::Create
        ),
        Err(Error::LimitExceeded("total bytes"))
    );
    unchanged(&s, &before);
}

#[test]
fn path_limits_apply_to_descendants_created_by_transfer() {
    let mut s = ResourceStore::new(
        CasePolicy::Sensitive,
        Limits {
            max_path_bytes: 8,
            max_depth: 3,
            ..Limits::default()
        },
    );
    s.write("a/b/file", [], WriteMode::Create).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.move_entry("a", "long"),
        Err(Error::LimitExceeded("path bytes"))
    );
    assert_eq!(s.copy("a", "x/y"), Err(Error::LimitExceeded("path bytes")));
    unchanged(&s, &before);
    let mut s = ResourceStore::new(
        CasePolicy::Sensitive,
        Limits {
            max_depth: 2,
            ..Limits::default()
        },
    );
    s.write("a/b", [], WriteMode::Create).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.move_entry("a", "x/y"),
        Err(Error::LimitExceeded("path depth"))
    );
    unchanged(&s, &before);
}

#[test]
fn strict_text_and_append_preserve_bytes_and_reject_invalid_utf8() {
    let mut s = ResourceStore::default();
    s.append_text("text", "Hello").unwrap();
    s.append_text("text", " 🌍\r\n").unwrap();
    assert_eq!(s.read_text("text").unwrap(), "Hello 🌍\r\n");
    s.write("binary", [0xff, 0x00], WriteMode::Create).unwrap();
    let before = s.snapshot();
    assert_eq!(s.read_text("binary"), Err(Error::InvalidUtf8));
    assert_eq!(s.append_text("binary", "x"), Err(Error::InvalidUtf8));
    unchanged(&s, &before);
    s.write_text("bom", "\u{feff}content", WriteMode::Create)
        .unwrap();
    assert!(s.read_text("bom").unwrap().starts_with('\u{feff}'));
}

#[test]
fn immutable_snapshots_can_be_read_on_other_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ResourceStore>();
    send_sync::<Snapshot>();
    let mut s = ResourceStore::default();
    s.write_text("a", "old", WriteMode::Create).unwrap();
    let snapshot = s.snapshot();
    let reader = std::thread::spawn(move || snapshot.read_text("a").unwrap());
    s.write_text("a", "new", WriteMode::Replace).unwrap();
    assert_eq!(reader.join().unwrap(), "old");
    assert_eq!(s.read_text("a").unwrap(), "new");
}

#[test]
fn deterministic_mutation_sequence_matches_a_flat_reference_model() {
    use std::collections::BTreeMap;
    let mut expected = BTreeMap::new();
    let mut s = ResourceStore::default();
    let mut seed = 42u64;
    for step in 0..500 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let path = format!("file{}", seed % 17);
        if seed & 4 == 0 {
            assert_eq!(
                s.remove(&path, false).unwrap(),
                expected.remove(&path).is_some()
            );
        } else {
            let bytes = format!("value{step}").into_bytes();
            s.write(&path, &bytes, WriteMode::Upsert).unwrap();
            expected.insert(path, bytes);
        }
        assert_eq!(s.len(), expected.len());
        assert_eq!(
            s.total_bytes(),
            expected.values().map(Vec::len).sum::<usize>()
        );
        for (path, bytes) in &expected {
            assert_eq!(&*s.read(path).unwrap(), bytes);
        }
    }
}

#[test]
fn every_created_node_has_distinct_identity_and_snapshot_keeps_old_addresses() {
    let mut s = ResourceStore::default();
    s.write_many(
        [("a/b", b"x".as_slice()), ("c/d", b"y".as_slice())],
        WriteMode::Create,
    )
    .unwrap();
    let old = s.snapshot();
    let original = s.entry("a/b").unwrap();
    s.copy("a", "copy").unwrap();
    s.move_entry("a", "moved").unwrap();
    let ids: std::collections::BTreeSet<_> = all(&s).iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), all(&s).len());
    assert_eq!(old.entry_by_id(original.id).unwrap().path, "a/b");
    assert_eq!(s.entry_by_id(original.id).unwrap().path, "moved/b");
}

#[test]
fn transfers_respect_destination_attributes_and_file_parents() {
    let mut s = ResourceStore::default();
    s.write("source", b"x", WriteMode::Create).unwrap();
    s.write("file", [], WriteMode::Create).unwrap();
    s.create_directory("locked").unwrap();
    s.set_attributes(
        "locked",
        Attributes {
            read_only: true,
            hidden: false,
        },
    )
    .unwrap();
    let before = s.snapshot();
    for moving in [false, true] {
        let result = if moving {
            s.move_entry("source", "locked/new")
        } else {
            s.copy("source", "locked/new")
        };
        assert_eq!(result, Err(Error::ReadOnly));
        let result = if moving {
            s.move_entry("source", "file/nested/new")
        } else {
            s.copy("source", "file/nested/new")
        };
        assert_eq!(result, Err(Error::NotDirectory));
        unchanged(&s, &before);
    }
}

#[test]
fn batch_panic_cannot_commit_partial_state() {
    let mut s = ResourceStore::default();
    let before = s.snapshot();
    let items = (0..2).map(|i| {
        assert_eq!(i, 0, "simulated producer failure");
        ("a", b"hello".as_slice())
    });
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        s.write_many(items, WriteMode::Create).unwrap();
    }));
    assert!(outcome.is_err());
    unchanged(&s, &before);
}

#[test]
fn moving_at_payload_budget_does_not_count_source_and_destination_twice() {
    let mut s = ResourceStore::new(
        CasePolicy::Sensitive,
        Limits {
            max_total_bytes: 3,
            ..Limits::default()
        },
    );
    let entry = s.write("source", b"abc", WriteMode::Create).unwrap();
    s.move_entry("source", "new/destination").unwrap();
    assert_eq!(s.total_bytes(), 3);
    assert_eq!(s.entry("new/destination").unwrap().id, entry.id);
}

#[test]
fn attribute_updates_invalidate_version_checked_writes() {
    let mut s = ResourceStore::default();
    let original = s.write("file", [], WriteMode::Create).unwrap();
    let changed = s
        .set_attributes(
            "file",
            Attributes {
                hidden: true,
                read_only: false,
            },
        )
        .unwrap();
    assert_eq!(
        s.write("file", [], WriteMode::CompareVersion(original.version)),
        Err(Error::VersionConflict {
            expected: original.version,
            actual: changed.version
        })
    );
    assert_eq!(original.id, changed.id);
}
