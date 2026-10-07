use rustlibrary_resource_store::{Attributes, CasePolicy, Limits, ResourceStore, WriteMode};
use rustlibrary_resource_store_fs::{
    Error, ExportOptions, ImportOptions, export_directory, import_directory,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Sandbox {
    root: PathBuf,
    parent: PathBuf,
}
impl Sandbox {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = std::env::temp_dir().canonicalize().unwrap();
        loop {
            let root = parent.join(format!(
                "rustlibrary-fs-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&root) {
                Ok(()) => return Self { root, parent },
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("create test directory: {e}"),
            }
        }
    }
    fn join(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        assert!(
            self.root.is_absolute()
                && self.root.starts_with(&self.parent)
                && self.root != self.parent
        );
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn options(limits: Limits) -> ImportOptions {
    ImportOptions {
        limits,
        ..ImportOptions::default()
    }
}
fn seed(root: &Path) {
    fs::create_dir_all(root.join("docs/empty")).unwrap();
    fs::create_dir_all(root.join("assets")).unwrap();
    fs::write(root.join("docs/main.txt"), "Hello 🌍\r\n").unwrap();
    fs::write(root.join("assets/data.bin"), [0, 255, 128, 1]).unwrap();
    fs::write(root.join(".hidden"), "included").unwrap();
}

#[test]
fn directory_roundtrip_preserves_bytes_and_empty_folders() {
    let temp = Sandbox::new();
    let input = temp.join("input");
    seed(&input);
    let imported = import_directory(&input, ImportOptions::default()).unwrap();
    assert_eq!(imported.len(), 3);
    let captured = imported.snapshot();
    let destination = temp.join("output");
    let report = export_directory(&captured, &destination, ExportOptions::default()).unwrap();
    assert_eq!(report.files, 3);
    assert_eq!(report.directories, 3);
    assert_eq!(report.bytes, imported.total_bytes());
    let again = import_directory(&destination, ImportOptions::default()).unwrap();
    for path in ["docs/main.txt", "assets/data.bin", ".hidden"] {
        assert_eq!(again.read(path).unwrap(), captured.read(path).unwrap());
    }
    assert!(destination.join("docs/empty").is_dir());
    assert_eq!(
        fs::read(input.join("docs/main.txt")).unwrap(),
        "Hello 🌍\r\n".as_bytes()
    );
}

#[test]
fn export_uses_captured_bytes_and_never_merges_existing_destination() {
    let temp = Sandbox::new();
    let mut store = ResourceStore::default();
    store.write_text("file", "old", WriteMode::Create).unwrap();
    let old = store.snapshot();
    store.write_text("file", "new", WriteMode::Replace).unwrap();
    let destination = temp.join("output");
    export_directory(&old, &destination, ExportOptions::default()).unwrap();
    assert_eq!(fs::read_to_string(destination.join("file")).unwrap(), "old");
    let error =
        export_directory(&store.snapshot(), &destination, ExportOptions::default()).unwrap_err();
    assert!(!error.destination_created);
    assert_eq!(fs::read_to_string(destination.join("file")).unwrap(), "old");
    fs::create_dir(temp.join("empty")).unwrap();
    assert!(
        !export_directory(&old, temp.join("empty"), ExportOptions::default())
            .unwrap_err()
            .destination_created
    );
}

#[test]
fn metadata_is_not_native_permission_or_hidden_policy() {
    let temp = Sandbox::new();
    let mut store = ResourceStore::default();
    store
        .write("secret/file", b"data", WriteMode::Create)
        .unwrap();
    store
        .set_attributes(
            "secret",
            Attributes {
                hidden: true,
                read_only: true,
            },
        )
        .unwrap();
    export_directory(
        &store.snapshot(),
        temp.join("all"),
        ExportOptions::default(),
    )
    .unwrap();
    let report = export_directory(
        &store.snapshot(),
        temp.join("visible"),
        ExportOptions {
            include_hidden: false,
        },
    )
    .unwrap();
    assert_eq!(report.files, 0);
    assert!(!temp.join("visible/secret").exists());
    let imported = import_directory(temp.join("all"), ImportOptions::default()).unwrap();
    assert_eq!(
        imported.entry("secret").unwrap().attributes,
        Attributes::default()
    );
    assert_eq!(imported.read("secret/file").unwrap().as_ref(), b"data");
}

#[test]
fn export_preflight_rejects_nonportable_names_and_ascii_case_collisions() {
    let temp = Sandbox::new();
    for (i, bad) in ["CON.txt", "a.", "x*y", "dir/LPT1", "name "]
        .iter()
        .enumerate()
    {
        let mut store = ResourceStore::default();
        store.write(bad, [], WriteMode::Create).unwrap();
        let destination = temp.join(&format!("bad{i}"));
        let error = export_directory(&store.snapshot(), &destination, ExportOptions::default())
            .unwrap_err();
        assert!(matches!(error.error, Error::InvalidName(_)));
        assert!(!error.destination_created && !destination.exists());
    }
    let mut store = ResourceStore::default();
    store.write("Dir/a", [], WriteMode::Create).unwrap();
    store.write("dir/b", [], WriteMode::Create).unwrap();
    let error = export_directory(
        &store.snapshot(),
        temp.join("collision"),
        ExportOptions::default(),
    )
    .unwrap_err();
    assert!(matches!(error.error, Error::NameCollision(_)));
    assert!(!error.destination_created);
}

#[test]
fn export_missing_parent_does_not_create_ancestors() {
    let temp = Sandbox::new();
    let snapshot = ResourceStore::default().snapshot();
    let error = export_directory(
        &snapshot,
        temp.join("absent/output"),
        ExportOptions::default(),
    )
    .unwrap_err();
    assert!(!error.destination_created);
    assert!(!temp.join("absent").exists());
}

#[test]
fn export_reports_partial_destination_on_native_io_failure() {
    let temp = Sandbox::new();
    let mut store = ResourceStore::new(
        CasePolicy::Sensitive,
        Limits {
            max_path_bytes: 8192,
            ..Limits::default()
        },
    );
    store.write("a", b"first", WriteMode::Create).unwrap();
    // OS component/path limits exceed the logical-name preflight's remit.
    store
        .write(&"z".repeat(5000), [], WriteMode::Create)
        .unwrap();
    let destination = temp.join("partial");
    let error =
        export_directory(&store.snapshot(), &destination, ExportOptions::default()).unwrap_err();
    assert!(error.destination_created);
    assert!(matches!(error.error, Error::Io { .. }));
    assert_eq!(fs::read(destination.join("a")).unwrap(), b"first");
}

#[test]
fn imports_enforce_file_total_entry_depth_and_path_budgets() {
    let temp = Sandbox::new();
    seed(&temp.join("source"));
    let cases = [
        Limits {
            max_resource_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_total_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_entries: 1,
            ..Limits::default()
        },
        Limits {
            max_depth: 1,
            ..Limits::default()
        },
        Limits {
            max_path_bytes: 3,
            ..Limits::default()
        },
    ];
    for limits in cases {
        assert!(matches!(
            import_directory(temp.join("source"), options(limits)),
            Err(Error::Store(
                rustlibrary_resource_store::Error::LimitExceeded(_)
            ))
        ));
    }
    assert!(temp.join("source/docs/empty").is_dir());
    assert_eq!(
        fs::read(temp.join("source/assets/data.bin")).unwrap(),
        [0, 255, 128, 1]
    );
}

#[test]
fn empty_tree_and_zero_byte_files_respect_zero_byte_limits() {
    let temp = Sandbox::new();
    fs::create_dir(temp.join("source")).unwrap();
    let mut limits = Limits {
        max_total_bytes: 0,
        max_resource_bytes: 0,
        max_entries: 0,
        ..Limits::default()
    };
    assert!(
        import_directory(temp.join("source"), options(limits))
            .unwrap()
            .is_empty()
    );
    fs::write(temp.join("source/empty"), []).unwrap();
    limits.max_entries = 1;
    assert_eq!(
        import_directory(temp.join("source"), options(limits))
            .unwrap()
            .len(),
        1
    );
    fs::write(temp.join("source/empty"), [1]).unwrap();
    assert!(import_directory(temp.join("source"), options(limits)).is_err());
}

#[test]
fn import_rejects_file_or_missing_root_and_normalizes_explicit_case_policy() {
    let temp = Sandbox::new();
    fs::write(temp.join("file"), "x").unwrap();
    assert!(matches!(
        import_directory(temp.join("file"), ImportOptions::default()),
        Err(Error::UnsupportedEntry(_))
    ));
    assert!(matches!(
        import_directory(temp.join("absent"), ImportOptions::default()),
        Err(Error::Io { .. })
    ));
    fs::create_dir(temp.join("source")).unwrap();
    fs::write(temp.join("source/UPPER.txt"), "text").unwrap();
    let store = import_directory(
        temp.join("source"),
        ImportOptions {
            case_policy: CasePolicy::AsciiInsensitive,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(store.entry("upper.txt").unwrap().path, "upper.txt");
}

#[cfg(unix)]
#[test]
fn import_rejects_links_special_files_non_utf8_and_case_aliases() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let temp = Sandbox::new();
    fs::create_dir(temp.join("source")).unwrap();
    fs::write(temp.join("outside"), "do not follow").unwrap();
    symlink(temp.join("outside"), temp.join("source/link")).unwrap();
    assert!(matches!(
        import_directory(temp.join("source"), ImportOptions::default()),
        Err(Error::UnsupportedEntry(_))
    ));
    fs::remove_file(temp.join("source/link")).unwrap();
    symlink(temp.join("source"), temp.join("root-link")).unwrap();
    assert!(matches!(
        import_directory(temp.join("root-link"), ImportOptions::default()),
        Err(Error::UnsupportedEntry(_))
    ));
    for suffix in ["/", "/."] {
        let path = format!("{}{}", temp.join("root-link").display(), suffix);
        assert!(matches!(
            import_directory(path, ImportOptions::default()),
            Err(Error::UnsupportedEntry(_))
        ));
    }
    let invalid = temp
        .join("source")
        .join(std::ffi::OsString::from_vec(vec![0xff]));
    fs::write(&invalid, []).unwrap();
    assert!(matches!(
        import_directory(temp.join("source"), ImportOptions::default()),
        Err(Error::InvalidName(_))
    ));
    fs::remove_file(invalid).unwrap();
    let socket = std::os::unix::net::UnixListener::bind(temp.join("source/socket")).unwrap();
    assert!(matches!(
        import_directory(temp.join("source"), ImportOptions::default()),
        Err(Error::UnsupportedEntry(_))
    ));
    drop(socket);
    fs::remove_file(temp.join("source/socket")).unwrap();
    fs::write(temp.join("source/A"), []).unwrap();
    fs::write(temp.join("source/a"), []).unwrap();
    assert!(matches!(
        import_directory(
            temp.join("source"),
            ImportOptions {
                case_policy: CasePolicy::AsciiInsensitive,
                ..ImportOptions::default()
            }
        ),
        Err(Error::NameCollision(_))
    ));
}

#[cfg(windows)]
#[test]
fn windows_junction_is_rejected_without_traversal() {
    let temp = Sandbox::new();
    fs::create_dir(temp.join("source")).unwrap();
    fs::create_dir(temp.join("outside")).unwrap();
    fs::write(temp.join("outside/sentinel"), "untouched").unwrap();
    let link = temp.join("source/junction");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(temp.join("outside"))
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "junction fixture: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(matches!(
        import_directory(temp.join("source"), ImportOptions::default()),
        Err(Error::UnsupportedEntry(_))
    ));
    assert!(matches!(
        import_directory(&link, ImportOptions::default()),
        Err(Error::UnsupportedEntry(_))
    ));
    assert!(
        !export_directory(
            &ResourceStore::default().snapshot(),
            link.join("output"),
            ExportOptions::default()
        )
        .unwrap_err()
        .destination_created
    );
    assert_eq!(
        fs::read_to_string(temp.join("outside/sentinel")).unwrap(),
        "untouched"
    );
    // Remove only the verified fixture link; never recursively delete its target.
    assert!(link.starts_with(&temp.root));
    fs::remove_dir(link).unwrap();
}
