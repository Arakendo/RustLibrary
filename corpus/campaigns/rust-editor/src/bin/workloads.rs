//! Deterministic model-based editor workloads. This is corpus-owned validation,
//! not production editor code or a throughput benchmark.
use rustlibrary_data_structures::TextRope;
use rustlibrary_diff::{Limits as DiffLimits, diff_lines};
use rustlibrary_resource_store::{
    Attributes, CasePolicy, EntryId, EntryKind, Error, Limits, ListOptions, ResourceStore,
    Snapshot, WriteMode,
};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RESOURCE_BYTES: usize = 16 * 1024;

#[derive(Clone)]
struct ExpectedFile {
    bytes: Vec<u8>,
    id: EntryId,
    version: u64,
}
#[derive(Clone, Default)]
struct Model {
    files: BTreeMap<String, ExpectedFile>,
    directories: BTreeSet<String>,
}
impl Model {
    fn parents(&mut self, path: &str) {
        for (index, _) in path.match_indices('/') {
            self.directories.insert(path[..index].to_owned());
        }
    }
}
#[derive(Clone, Copy)]
struct Profile {
    name: &'static str,
    documents: usize,
    operations: usize,
}
const STANDARD: Profile = Profile {
    name: "standard",
    documents: 256,
    operations: 512,
};

#[derive(Debug, Eq, PartialEq)]
struct Report {
    profile: &'static str,
    policy: &'static str,
    documents: usize,
    operations: usize,
    checked_states: usize,
    retained_snapshots: usize,
    rejected_operations: usize,
    final_files: usize,
    final_bytes: usize,
}
fn require(condition: bool, message: &'static str) -> Result<()> {
    if !condition {
        return Err(std::io::Error::other(message).into());
    }
    Ok(())
}

/// The reference model stores owned Vec bytes and folder names. It does not
/// normalize through the store, use store prefixes, or delegate its mutations.
fn verify(view: &Snapshot, model: &Model) -> Result<()> {
    let entries = view.list(
        "",
        ListOptions {
            recursive: true,
            include_hidden: true,
        },
    )?;
    let actual_files: BTreeSet<_> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::File)
        .map(|e| e.path.as_str())
        .collect();
    let actual_dirs: BTreeSet<_> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Directory)
        .map(|e| e.path.as_str())
        .collect();
    require(
        actual_files == model.files.keys().map(String::as_str).collect(),
        "file address set differs from reference model",
    )?;
    require(
        actual_dirs == model.directories.iter().map(String::as_str).collect(),
        "directory set differs from reference model",
    )?;
    let ids: BTreeSet<_> = entries.iter().map(|e| e.id).collect();
    require(
        ids.len() == entries.len(),
        "entry identities are not unique within store",
    )?;
    require(
        view.len() == model.files.len(),
        "file count differs from reference model",
    )?;
    require(
        view.total_bytes() == model.files.values().map(|f| f.bytes.len()).sum::<usize>(),
        "byte accounting differs from reference model",
    )?;
    for (path, expected) in &model.files {
        let actual = view.entry(path)?;
        require(
            view.read(path)?.as_ref() == expected.bytes,
            "stored bytes differ from reference model",
        )?;
        require(
            actual.id == expected.id && actual.version == expected.version,
            "identity/version differs from tracked lifecycle",
        )?;
        require(
            actual.size == expected.bytes.len(),
            "metadata size differs from expected payload",
        )?;
    }
    Ok(())
}
fn file_from(store: &ResourceStore, path: &str, bytes: Vec<u8>) -> Result<ExpectedFile> {
    let entry = store.entry(path)?;
    Ok(ExpectedFile {
        bytes,
        id: entry.id,
        version: entry.version,
    })
}
fn workload(profile: Profile, policy: CasePolicy, label: &'static str) -> Result<Report> {
    let mut store = ResourceStore::new(
        policy,
        Limits {
            max_entries: profile.documents * 4 + profile.operations * 4,
            max_resource_bytes: RESOURCE_BYTES,
            max_total_bytes: 16 * 1024 * 1024,
            ..Limits::default()
        },
    );
    let mut model = Model::default();
    for document in 0..profile.documents {
        let path = format!("project/group{:02}/doc{document:04}.txt", document % 8);
        let bytes = format!("Document {document}\r\nHello 🌍 — 資料\r\n").into_bytes();
        store.write(&path, &bytes, WriteMode::Create)?;
        model.parents(&path);
        model
            .files
            .insert(path.clone(), file_from(&store, &path, bytes)?);
    }
    store.create_directory("project/empty")?;
    model.directories.insert("project/empty".into());
    verify(&store, &model)?;
    let mut captures = vec![(store.snapshot(), model.clone())];
    let mut rejected = 0;
    let mut checked = 1;
    let mut seed = 0x5eed_u64;
    for step in 0..profile.operations {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let path = model
            .files
            .keys()
            .nth((seed as usize) % model.files.len())
            .unwrap()
            .clone();
        let previous = model.files[&path].clone();
        match step % 8 {
            0 => {
                let before = String::from_utf8(previous.bytes.clone())?;
                let prefix = format!("Edit {step}: 🦀\n");
                let mut rope = TextRope::new(&before, RESOURCE_BYTES)?;
                rope.replace(0..0, &prefix)?;
                let expected = format!("{prefix}{before}").into_bytes();
                require(
                    rope.to_text().as_bytes() == expected,
                    "rope edit differs from independent string concatenation",
                )?;
                let changes = diff_lines(&before, &rope.to_text(), DiffLimits::default())?;
                require(
                    changes.len() == 1
                        && changes[0].old_bytes.is_empty()
                        && changes[0].new_bytes == (0..prefix.len()),
                    "diff does not describe expected prefix insertion",
                )?;
                let saved = store.write(
                    &path,
                    &expected,
                    WriteMode::CompareVersion(previous.version),
                )?;
                require(
                    saved.id == previous.id && saved.version > previous.version,
                    "save changed identity or failed to advance version",
                )?;
                let generation = store.generation();
                require(
                    matches!(
                        store.write(&path, b"stale", WriteMode::CompareVersion(previous.version)),
                        Err(Error::VersionConflict { .. })
                    ),
                    "stale save was accepted",
                )?;
                require(
                    store.generation() == generation,
                    "stale save changed generation",
                )?;
                rejected += 1;
                model.files.insert(
                    path,
                    ExpectedFile {
                        bytes: expected,
                        id: saved.id,
                        version: saved.version,
                    },
                );
            }
            1 => {
                let destination = format!("renamed/step{step:04}.txt");
                let moved = store.move_entry(&path, &destination)?;
                require(
                    moved.id == previous.id && moved.version > previous.version,
                    "move lost stable identity/version",
                )?;
                model.files.remove(&path);
                model.parents(&destination);
                model.files.insert(
                    destination,
                    ExpectedFile {
                        version: moved.version,
                        ..previous
                    },
                );
            }
            2 => {
                let destination = format!("copies/step{step:04}.txt");
                let copied = store.copy(&path, &destination)?;
                require(copied.id != previous.id, "copy reused source identity")?;
                model.parents(&destination);
                model.files.insert(
                    destination,
                    ExpectedFile {
                        id: copied.id,
                        version: copied.version,
                        bytes: previous.bytes,
                    },
                );
            }
            3 => {
                store.remove(&path, false)?;
                let replacement = format!("Recreated at step {step}\n").into_bytes();
                let entry = store.write(&path, &replacement, WriteMode::Create)?;
                require(
                    entry.id != previous.id,
                    "recreation reused deleted identity",
                )?;
                require(
                    matches!(store.entry_by_id(previous.id), Err(Error::NotFound)),
                    "deleted ID resolved to replacement",
                )?;
                model.files.insert(
                    path,
                    ExpectedFile {
                        id: entry.id,
                        version: entry.version,
                        bytes: replacement,
                    },
                );
            }
            4 => {
                let generation = store.generation();
                let new_path = format!("rolled-back/step{step:04}.txt");
                require(
                    store.write_many(
                        [
                            (new_path.as_str(), b"temporary".as_slice()),
                            (path.as_str(), b"collision".as_slice()),
                        ],
                        WriteMode::Create,
                    ) == Err(Error::AlreadyExists),
                    "collision batch was accepted",
                )?;
                require(
                    store.generation() == generation,
                    "failed batch changed generation",
                )?;
                rejected += 1;
            }
            5 => {
                let directory = path.rsplit_once('/').unwrap().0;
                store.set_attributes(
                    directory,
                    Attributes {
                        hidden: true,
                        read_only: true,
                    },
                )?;
                let generation = store.generation();
                require(
                    store.remove(directory, true) == Err(Error::ReadOnly),
                    "protected subtree deletion was accepted",
                )?;
                require(
                    store.generation() == generation,
                    "failed protected deletion changed generation",
                )?;
                require(
                    store
                        .list(
                            directory,
                            ListOptions {
                                recursive: true,
                                include_hidden: false,
                            },
                        )?
                        .is_empty(),
                    "hidden ancestor leaked visible descendants",
                )?;
                store.set_attributes(directory, Attributes::default())?;
                rejected += 1;
            }
            6 => {
                let generation = store.generation();
                require(
                    store.write(
                        "oversized.bin",
                        vec![0; RESOURCE_BYTES + 1],
                        WriteMode::Create,
                    ) == Err(Error::LimitExceeded("resource bytes")),
                    "oversized write was accepted",
                )?;
                require(
                    store.generation() == generation,
                    "rejected size limit changed generation",
                )?;
                rejected += 1;
            }
            _ => {
                let directory = format!("empty/step{step:04}");
                store.create_directory(&directory)?;
                model.parents(&directory);
                model.directories.insert(directory);
            }
        }
        verify(&store, &model)?;
        checked += 1;
        if (step + 1) % 128 == 0 {
            captures.push((store.snapshot(), model.clone()));
        }
    }
    // Captures contain the full expected state from their capture point, including
    // bytes no longer reachable from the live store.
    for (snapshot, expected) in &captures {
        verify(snapshot, expected)?;
        checked += 1;
    }
    Ok(Report {
        profile: profile.name,
        policy: label,
        documents: profile.documents,
        operations: profile.operations,
        checked_states: checked,
        retained_snapshots: captures.len(),
        rejected_operations: rejected,
        final_files: store.len(),
        final_bytes: store.total_bytes(),
    })
}
fn run(profile: Profile) -> Result<Vec<Report>> {
    Ok(vec![
        workload(profile, CasePolicy::Sensitive, "sensitive")?,
        workload(profile, CasePolicy::AsciiInsensitive, "ascii-insensitive")?,
    ])
}
fn render(reports: &[Report]) -> String {
    let mut text = String::from(
        "campaign\tprofile\tcase_policy\tinitial_documents\toperations\tchecked_states\tretained_snapshots\trejected_operations\tfinal_files\tfinal_bytes\toutcome\n",
    );
    for r in reports {
        text.push_str(&format!(
            "rust-editor-workloads\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\tpass\n",
            r.profile,
            r.policy,
            r.documents,
            r.operations,
            r.checked_states,
            r.retained_snapshots,
            r.rejected_operations,
            r.final_files,
            r.final_bytes
        ));
    }
    text
}
fn main() -> Result<()> {
    print!("{}", render(&run(STANDARD)?));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    const SMALL: Profile = Profile {
        name: "test",
        documents: 16,
        operations: 32,
    };
    #[test]
    fn lifecycle_sequences_match_model_under_both_policies() -> Result<()> {
        let reports = run(SMALL)?;
        for report in reports {
            assert_eq!(report.rejected_operations, 16);
            assert_eq!(report.final_files, 20);
            assert_eq!(report.retained_snapshots, 1);
        }
        Ok(())
    }
    #[test]
    fn repeated_runs_produce_identical_observations() -> Result<()> {
        assert_eq!(render(&run(SMALL)?), render(&run(SMALL)?));
        Ok(())
    }
    #[test]
    fn reference_model_detects_changed_bytes_identity_and_accounting() -> Result<()> {
        let mut store = ResourceStore::default();
        store.write("a", b"original", WriteMode::Create)?;
        let mut model = Model::default();
        model
            .files
            .insert("a".into(), file_from(&store, "a", b"original".to_vec())?);
        verify(&store, &model)?;
        let mut wrong_bytes = model.clone();
        wrong_bytes.files.get_mut("a").unwrap().bytes = b"tampered".to_vec();
        assert!(verify(&store, &wrong_bytes).is_err());
        let mut wrong_id = model.clone();
        wrong_id.files.get_mut("a").unwrap().id = store.entry("")?.id;
        assert!(verify(&store, &wrong_id).is_err());
        let mut wrong_size = model.clone();
        wrong_size.files.get_mut("a").unwrap().bytes.push(0);
        assert!(verify(&store, &wrong_size).is_err());
        verify(&store, &model)?;
        Ok(())
    }
}
