//! Campaign-owned editor orchestration. All storage, text edits, and comparison
//! use public RustLibrary APIs; no production RustEditor implementation is imported.
use rustlibrary_data_structures::TextRope;
use rustlibrary_diff::{Limits as DiffLimits, diff_lines};
use rustlibrary_resource_store::{
    Attributes, CasePolicy, EntryId, Error, Limits, ListOptions, ResourceStore, Snapshot, WriteMode,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = include_str!("../fixtures/document.txt");
const EDITED: &str = include_str!("../fixtures/edited.txt");

/// An editor owns its unsaved draft and captured version. This small consumer
/// deliberately handles one selected store; IDs never cross store boundaries.
struct OpenDocument {
    id: EntryId,
    base_version: u64,
    draft: TextRope,
}
impl OpenDocument {
    fn open(store: &ResourceStore, path: &str) -> Result<Self> {
        let entry = store.entry(path)?;
        Ok(Self {
            id: entry.id,
            base_version: entry.version,
            draft: TextRope::new(&store.read_text(path)?, 4096)?,
        })
    }

    fn save(&mut self, store: &mut ResourceStore) -> std::result::Result<(), Error> {
        let entry = store.entry_by_id(self.id)?;
        let saved = store.write_text(
            &entry.path,
            &self.draft.to_text(),
            WriteMode::CompareVersion(self.base_version),
        )?;
        self.base_version = saved.version;
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct Observation {
    policy: &'static str,
    control: &'static str,
}
fn record(
    observations: &mut Vec<Observation>,
    policy: &'static str,
    control: &'static str,
    passed: bool,
) -> Result<()> {
    if !passed {
        return Err(std::io::Error::other(format!("campaign failed: {policy}/{control}")).into());
    }
    observations.push(Observation { policy, control });
    Ok(())
}

fn same_state(left: &Snapshot, right: &Snapshot) -> Result<bool> {
    let options = ListOptions {
        recursive: true,
        include_hidden: true,
    };
    let entries = left.list("", options)?;
    if left.generation() != right.generation() || entries != right.list("", options)? {
        return Ok(false);
    }
    for entry in entries {
        if entry.kind == rustlibrary_resource_store::EntryKind::File
            && left.read(&entry.path)? != right.read(&entry.path)?
        {
            return Ok(false);
        }
    }
    Ok(left.total_bytes() == right.total_bytes())
}

fn scenario(policy: CasePolicy, label: &'static str) -> Result<Vec<Observation>> {
    let mut observations = Vec::new();
    let mut store = ResourceStore::new(
        policy,
        Limits {
            max_entries: 64,
            max_total_bytes: 8192,
            max_resource_bytes: 4096,
            ..Limits::default()
        },
    );
    store.create_directory("drafts")?;
    store.write_many(
        [
            ("docs/main.txt", SOURCE.as_bytes()),
            ("docs/includes/note.txt", b"related content\n".as_slice()),
            ("assets/icon.bin", b"\x89PNG".as_slice()),
        ],
        WriteMode::Create,
    )?;
    let related = store.resolve("docs/main.txt", "includes/note.txt")?;
    record(
        &mut observations,
        label,
        "bundle-navigation",
        store.read_text(&related)? == "related content\n"
            && store.list("drafts", ListOptions::default())?.is_empty(),
    )?;

    let mut first = OpenDocument::open(&store, "docs/main.txt")?;
    let mut stale = OpenDocument::open(&store, "docs/main.txt")?;
    let preview = store.snapshot();
    // UTF-16 columns are editor-facing; the rope translates to scalar-safe bytes.
    let start = first.draft.line_utf16_to_byte(1, 0)?;
    let end = first.draft.line_utf16_to_byte(1, 8)?;
    first.draft.replace(start..end, "Hello Rust")?;
    record(
        &mut observations,
        label,
        "draft-isolation",
        first.draft.to_text() == EDITED && store.read_text("docs/main.txt")? == SOURCE,
    )?;

    let changes = diff_lines(SOURCE, &first.draft.to_text(), DiffLimits::default())?;
    record(
        &mut observations,
        label,
        "shared-text-comparison",
        changes.len() == 1 && changes[0].old_lines == (1..2) && changes[0].new_lines == (1..2),
    )?;

    first.save(&mut store)?;
    record(
        &mut observations,
        label,
        "version-checked-save",
        store.read_text("docs/main.txt")? == EDITED,
    )?;
    record(
        &mut observations,
        label,
        "preview-snapshot",
        preview.read_text("docs/main.txt")? == SOURCE,
    )?;

    stale.draft.replace(0..0, "unsaved local change\n")?;
    let before = store.snapshot();
    let rejected = matches!(stale.save(&mut store), Err(Error::VersionConflict { .. }));
    record(
        &mut observations,
        label,
        "stale-save-rejected",
        rejected
            && same_state(&store, &before)?
            && stale.draft.to_text().starts_with("unsaved local change\n"),
    )?;

    store.move_entry("docs", "project/docs")?;
    let moved = store.entry_by_id(first.id)?;
    record(
        &mut observations,
        label,
        "open-tab-follows-identity",
        moved.path == "project/docs/main.txt"
            && preview.entry_by_id(first.id)?.path == "docs/main.txt",
    )?;
    let before = store.snapshot();
    // Rename changes the version: finding the current path must not silently
    // bless the draft. The consumer explicitly reloads after observing conflict.
    record(
        &mut observations,
        label,
        "rename-requires-refresh",
        matches!(first.save(&mut store), Err(Error::VersionConflict { .. }))
            && same_state(&store, &before)?,
    )?;
    first = OpenDocument::open(&store, &moved.path)?;
    first.draft.replace(
        first.draft.len_bytes()..first.draft.len_bytes(),
        "saved after rename\n",
    )?;
    first.save(&mut store)?;

    store.copy("project/docs", "backup/docs")?;
    let copied = store.entry("backup/docs/main.txt")?;
    record(
        &mut observations,
        label,
        "backup-copy-identity",
        copied.id != first.id
            && store.content_equals("backup/docs/main.txt", "project/docs/main.txt")?,
    )?;
    store.set_attributes(
        "backup",
        Attributes {
            hidden: true,
            read_only: true,
        },
    )?;
    record(
        &mut observations,
        label,
        "hidden-navigation",
        !store
            .list("", ListOptions::default())?
            .iter()
            .any(|entry| entry.path == "backup")
            && store
                .read_text("backup/docs/main.txt")?
                .ends_with("saved after rename\n"),
    )?;
    let before = store.snapshot();
    record(
        &mut observations,
        label,
        "protected-subtree-rollback",
        store.remove("backup", true) == Err(Error::ReadOnly) && same_state(&store, &before)?,
    )?;

    let before = store.snapshot();
    let collision = store.write_many(
        [
            ("incoming/new.txt", b"new".as_slice()),
            ("project/docs/main.txt", b"collision".as_slice()),
        ],
        WriteMode::Create,
    );
    record(
        &mut observations,
        label,
        "batch-import-rollback",
        collision == Err(Error::AlreadyExists) && same_state(&store, &before)?,
    )?;

    let before = store.snapshot();
    record(
        &mut observations,
        label,
        "bounded-import-rollback",
        store.write("incoming/large.bin", vec![0; 4097], WriteMode::Create)
            == Err(Error::LimitExceeded("resource bytes"))
            && same_state(&store, &before)?,
    )?;

    let before = store.snapshot();
    record(
        &mut observations,
        label,
        "reference-root-confinement",
        store.resolve("project/docs/main.txt", "../../../outside.txt") == Err(Error::InvalidPath)
            && same_state(&store, &before)?,
    )?;

    let path = store.entry_by_id(first.id)?.path;
    store.remove(&path, false)?;
    let replacement = store.write_text(&path, "replacement", WriteMode::Create)?;
    let before = store.snapshot();
    record(
        &mut observations,
        label,
        "deleted-tab-cannot-save-over-replacement",
        first.id != replacement.id
            && first.save(&mut store) == Err(Error::NotFound)
            && same_state(&store, &before)?,
    )?;

    record(
        &mut observations,
        label,
        "case-policy-visible",
        store.contains("PROJECT/DOCS/MAIN.TXT")? == (policy == CasePolicy::AsciiInsensitive),
    )?;
    Ok(observations)
}

fn run() -> Result<Vec<Observation>> {
    let mut observations = scenario(CasePolicy::Sensitive, "sensitive")?;
    observations.extend(scenario(CasePolicy::AsciiInsensitive, "ascii-insensitive")?);
    Ok(observations)
}

fn report(observations: &[Observation]) -> String {
    let mut output = String::from("campaign\tcase_policy\tcontrol\toutcome\n");
    for observation in observations {
        output.push_str(&format!(
            "rust-editor\t{}\t{}\tpass\n",
            observation.policy, observation.control
        ));
    }
    output
}

fn main() -> Result<()> {
    // No successful report is emitted until every positive/negative control passes.
    print!("{}", report(&run()?));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_controls_pass_for_both_policies() -> Result<()> {
        let observations = run()?;
        assert_eq!(observations.len(), 32);
        Ok(())
    }

    #[test]
    fn observations_are_deterministic_across_fresh_sessions() -> Result<()> {
        assert_eq!(report(&run()?), report(&run()?));
        Ok(())
    }

    #[test]
    fn failed_control_does_not_emit_success() {
        let mut observations = Vec::new();
        assert!(record(&mut observations, "sensitive", "negative-control", false).is_err());
        assert!(observations.is_empty());
    }
}
