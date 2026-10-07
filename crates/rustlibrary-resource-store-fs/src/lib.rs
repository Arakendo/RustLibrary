//! Native adapters for caller-controlled, quiescent directory trees.
//!
//! Import returns a new store only on success. Export never merges or overwrites:
//! it requires an absent destination. These path-based operations are not a
//! sandbox against concurrent filesystem changes. See the README for ownership,
//! partial-export, naming, and link-handling contracts.

use rustlibrary_resource_store::{
    CasePolicy, EntryKind, Limits, ListOptions, ResourceStore, Snapshot, WriteMode,
};
use std::{
    collections::BTreeSet,
    fmt,
    fs::{self, File, Metadata, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

/// Native operation failures retain their OS or store cause without payload data.
#[derive(Debug)]
pub enum Error {
    Io { path: PathBuf, source: io::Error },
    Store(rustlibrary_resource_store::Error),
    UnsupportedEntry(PathBuf),
    InvalidName(PathBuf),
    NameCollision(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "filesystem operation at {}: {source}", path.display())
            }
            Self::Store(source) => write!(f, "{source}"),
            Self::UnsupportedEntry(path) => {
                write!(f, "unsupported filesystem entry: {}", path.display())
            }
            Self::InvalidName(path) => write!(f, "unsupported native name: {}", path.display()),
            Self::NameCollision(path) => write!(f, "native name collision: {path}"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Store(source) => Some(source),
            _ => None,
        }
    }
}
impl From<rustlibrary_resource_store::Error> for Error {
    fn from(source: rustlibrary_resource_store::Error) -> Self {
        Self::Store(source)
    }
}
fn at<T>(path: &Path, result: io::Result<T>) -> Result<T, Error> {
    result.map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })
}

/// Import budgets and logical case policy. Native attributes are not imported.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImportOptions {
    pub limits: Limits,
    pub case_policy: CasePolicy,
}

/// Visibility selection is explicit; the default exports all entries.
#[derive(Clone, Copy, Debug)]
pub struct ExportOptions {
    pub include_hidden: bool,
}
impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_hidden: true,
        }
    }
}

/// Counts exclude the destination root.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExportReport {
    pub files: usize,
    pub directories: usize,
    pub bytes: usize,
}

/// After destination creation, an I/O failure can leave partial output.
/// The adapter never automatically deletes that output.
#[derive(Debug)]
pub struct ExportError {
    pub error: Error,
    /// True only if this call successfully created the destination directory.
    pub destination_created: bool,
}
impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "export failed (destination_created={}): {}",
            self.destination_created, self.error
        )
    }
}
impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

fn linked(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT: reject junctions and other redirects too.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn inspect(path: &Path) -> Result<Metadata, Error> {
    let metadata = at(path, fs::symlink_metadata(path))?;
    if linked(&metadata) || !(metadata.is_file() || metadata.is_dir()) {
        return Err(Error::UnsupportedEntry(path.to_owned()));
    }
    Ok(metadata)
}

/// Conservative Windows/Unix component policy, applied on every platform.
fn valid_component(name: &str) -> bool {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*%#".contains(c))
    {
        return false;
    }
    let upper = name.split('.').next().unwrap_or("").to_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return false;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(tail) = upper.strip_prefix(prefix) {
            if matches!(
                tail,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            ) {
                return false;
            }
        }
    }
    true
}

/// Imports regular files and directories, including empty folders, into a new
/// store. Rejects links/reparse points, special files, non-UTF-8 names, and case
/// collisions. Caller must prevent concurrent changes to the source tree.
pub fn import_directory(
    source: impl AsRef<Path>,
    options: ImportOptions,
) -> Result<ResourceStore, Error> {
    // Remove trailing separators/dot syntax before inspecting the final node.
    // Otherwise a native trailing slash can force directory-following semantics.
    let source_path: PathBuf = source.as_ref().components().collect();
    let source = source_path.as_path();
    if !inspect(source)?.is_dir() {
        return Err(Error::UnsupportedEntry(source.to_owned()));
    }
    let mut store = ResourceStore::new(options.case_policy, options.limits);
    let mut pending = vec![(source.to_owned(), String::new())];
    let mut admitted = BTreeSet::new();
    let mut discovered = 0usize;
    while let Some((directory, logical_directory)) = pending.pop() {
        if !inspect(&directory)?.is_dir() {
            return Err(Error::UnsupportedEntry(directory));
        }
        let mut children = Vec::new();
        for child in at(&directory, fs::read_dir(&directory))? {
            let child = at(&directory, child)?;
            discovered = discovered
                .checked_add(1)
                .ok_or(rustlibrary_resource_store::Error::LimitExceeded("entries"))?;
            if discovered > options.limits.max_entries {
                return Err(rustlibrary_resource_store::Error::LimitExceeded("entries").into());
            }
            let path = child.path();
            let name = child
                .file_name()
                .into_string()
                .map_err(|_| Error::InvalidName(path.clone()))?;
            if !valid_component(&name) {
                return Err(Error::InvalidName(path));
            }
            let logical = if logical_directory.is_empty() {
                name
            } else {
                format!("{logical_directory}/{name}")
            };
            let logical = store.normalize_path(&logical)?;
            if !admitted.insert(logical.clone()) {
                return Err(Error::NameCollision(logical));
            }
            children.push((path, logical));
        }
        children.sort_by(|a, b| a.1.cmp(&b.1));
        for (path, logical) in children {
            let metadata = inspect(&path)?;
            if metadata.is_dir() {
                store.create_directory(&logical)?;
                pending.push((path, logical));
            } else {
                let budget = options
                    .limits
                    .max_resource_bytes
                    .min(options.limits.max_total_bytes - store.total_bytes());
                if metadata.len() > budget as u64 {
                    return Err(
                        rustlibrary_resource_store::Error::LimitExceeded("import bytes").into(),
                    );
                }
                let file = at(&path, File::open(&path))?;
                if !at(&path, file.metadata())?.is_file() {
                    return Err(Error::UnsupportedEntry(path));
                }
                let bytes = at(&path, bounded_read(file, budget))?;
                store.write(&logical, bytes, WriteMode::Create)?;
            }
        }
    }
    Ok(store)
}

// Check actual reads too: metadata length alone is not a byte budget.
fn bounded_read(reader: impl Read, budget: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((budget as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > budget {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "import byte budget exceeded during read",
        ));
    }
    Ok(bytes)
}

/// Exports an entire immutable snapshot into an absent destination whose parent
/// already exists. Preflight rejects nonportable names and ASCII case aliases.
/// Export copies bytes and directory shape, not IDs, versions, tags, or native
/// metadata. A failure after creating destination leaves explicit partial output.
pub fn export_directory(
    snapshot: &Snapshot,
    destination: impl AsRef<Path>,
    options: ExportOptions,
) -> Result<ExportReport, ExportError> {
    let destination = destination.as_ref();
    let mut created = false;
    let result = (|| -> Result<ExportReport, Error> {
        let entries = snapshot.list(
            "",
            ListOptions {
                recursive: true,
                include_hidden: options.include_hidden,
            },
        )?;
        let mut names = BTreeSet::new();
        for entry in &entries {
            if !entry.path.split('/').all(valid_component) {
                return Err(Error::InvalidName(PathBuf::from(&entry.path)));
            }
            if !names.insert(entry.path.to_ascii_lowercase()) {
                return Err(Error::NameCollision(entry.path.clone()));
            }
        }
        // Only the caller's destination may be absolute; resource components cannot.
        if destination.file_name().is_none() {
            return Err(Error::InvalidName(destination.to_owned()));
        }
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        if !inspect(parent)?.is_dir() {
            return Err(Error::UnsupportedEntry(parent.to_owned()));
        }
        at(destination, fs::create_dir(destination))?;
        created = true;
        let mut report = ExportReport::default();
        for entry in entries {
            let path = destination.join(&entry.path);
            if entry.kind == EntryKind::Directory {
                at(&path, fs::create_dir(&path))?;
                report.directories += 1;
            } else {
                let bytes = snapshot.read(&entry.path)?;
                let mut file = at(
                    &path,
                    OpenOptions::new().write(true).create_new(true).open(&path),
                )?;
                at(&path, file.write_all(&bytes))?;
                report.files += 1;
                report.bytes += bytes.len();
            }
        }
        Ok(report)
    })();
    result.map_err(|error| ExportError {
        error,
        destination_created: created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_reads_are_bounded_even_without_metadata() {
        assert!(bounded_read(&b"1234"[..], 3).is_err());
        assert_eq!(bounded_read(&b"123"[..], 3).unwrap(), b"123");
        assert_eq!(bounded_read(&b""[..], 0).unwrap(), b"");
        assert!(bounded_read(&b"x"[..], 0).is_err());
    }

    #[test]
    fn conservative_names_reject_devices_and_path_aliases() {
        for bad in [
            "CON",
            "aux.txt",
            "COM1.log",
            "LPT²",
            "NUL.tar.gz",
            "CONIN$",
            "x.",
            "x ",
            "x:y",
            "x/y",
            "x\\y",
            "x\0y",
            "x?y",
            "x*y",
            ".",
            "..",
        ] {
            assert!(!valid_component(bad), "{bad:?}");
        }
        for good in [
            "document.xml",
            "COM10",
            "auxiliary",
            ".hidden",
            "資料",
            "report 1.txt",
        ] {
            assert!(valid_component(good), "{good:?}");
        }
    }
}
