//! A separate native workflow: temporary source -> resource store -> draft ->
//! captured output -> new native directory -> reopened resource store.
use rustlibrary_data_structures::TextRope;
use rustlibrary_resource_store::WriteMode;
use rustlibrary_resource_store_fs::{
    ExportOptions, ImportOptions, export_directory, import_directory,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct TemporaryWorkspace {
    path: PathBuf,
    parent: PathBuf,
}
impl TemporaryWorkspace {
    fn create() -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = std::env::temp_dir().canonicalize()?;
        loop {
            let path = parent.join(format!(
                "rustlibrary-native-corpus-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path, parent }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
    fn cleanup(&self) -> Result<()> {
        if !self.path.is_absolute()
            || !self.path.starts_with(&self.parent)
            || self.path == self.parent
        {
            return Err(std::io::Error::other("temporary workspace escaped its parent").into());
        }
        fs::remove_dir_all(&self.path)?;
        Ok(())
    }
}
impl Drop for TemporaryWorkspace {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = self.cleanup();
        }
    }
}
fn check(condition: bool, control: &'static str, rows: &mut Vec<&'static str>) -> Result<()> {
    if !condition {
        return Err(std::io::Error::other(format!("native campaign failed: {control}")).into());
    }
    rows.push(control);
    Ok(())
}
fn run() -> Result<Vec<&'static str>> {
    let workspace = TemporaryWorkspace::create()?;
    let input = workspace.path.join("input");
    let output = workspace.path.join("output");
    fs::create_dir_all(input.join("docs/empty"))?;
    let original = include_str!("../../fixtures/document.txt");
    fs::write(input.join("docs/main.txt"), original)?;
    let mut store = import_directory(&input, ImportOptions::default())?;
    let mut rows = Vec::new();
    check(
        store.read_text("docs/main.txt")? == original && store.contains("docs/empty")?,
        "import-source-and-empty-folder",
        &mut rows,
    )?;

    let entry = store.entry("docs/main.txt")?;
    let mut draft = TextRope::new(&store.read_text("docs/main.txt")?, 4096)?;
    draft.replace(0..0, "Edited in the corpus\n")?;
    store.write_text(
        "docs/main.txt",
        &draft.to_text(),
        WriteMode::CompareVersion(entry.version),
    )?;
    check(
        fs::read_to_string(input.join("docs/main.txt"))? == original,
        "draft-save-leaves-source-on-disk-unchanged",
        &mut rows,
    )?;

    let saved = store.snapshot();
    store.write_text("docs/main.txt", "later unsaved output", WriteMode::Replace)?;
    let report = export_directory(&saved, &output, ExportOptions::default())?;
    check(
        report.files == 1 && output.join("docs/empty").is_dir(),
        "export-captured-output",
        &mut rows,
    )?;
    let reopened = import_directory(&output, ImportOptions::default())?;
    check(
        reopened.read_text("docs/main.txt")? == draft.to_text(),
        "reopen-captured-text",
        &mut rows,
    )?;
    let rejected =
        export_directory(&store.snapshot(), &output, ExportOptions::default()).unwrap_err();
    check(
        !rejected.destination_created
            && fs::read_to_string(output.join("docs/main.txt"))? == draft.to_text(),
        "reject-output-overwrite",
        &mut rows,
    )?;

    workspace.cleanup()?;
    check(
        !workspace.path.exists(),
        "cleanup-owned-temporary-workspace",
        &mut rows,
    )?;
    Ok(rows)
}
fn main() -> Result<()> {
    let rows = run()?;
    println!("campaign\tcontrol\toutcome");
    for row in rows {
        println!("rust-editor-native\t{row}\tpass");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_workflow_roundtrips_and_cleans_up() -> Result<()> {
        assert_eq!(run()?.len(), 6);
        Ok(())
    }
}
