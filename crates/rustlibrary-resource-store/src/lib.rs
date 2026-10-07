//! In-memory resource bundles with explicit replacement and immutable snapshots.
//!
//! Addresses are root-relative logical paths, not URLs or host filesystem paths.
//! The empty address denotes the permanent root. See the crate README for limits,
//! case policy, mutation guarantees, and differences from C# MemoryStore.
//!
//!     use rustlibrary_resource_store::{ResourceStore, WriteMode};
//!     let mut store = ResourceStore::default();
//!     store.write_text("docs/readme.txt", "Hello", WriteMode::Create)?;
//!     let snapshot = store.snapshot();
//!     store.write_text("docs/readme.txt", "Updated", WriteMode::Replace)?;
//!     assert_eq!(snapshot.read_text("docs/readme.txt")?, "Hello");
//!     # Ok::<(), rustlibrary_resource_store::Error>(())

use std::{collections::BTreeMap, fmt, io::Cursor, ops::Deref, sync::Arc};
mod path;
use path::{below, parent};

/// Stable failures. Errors never contain resource payloads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidPath,
    NotFound,
    AlreadyExists,
    NotDirectory,
    IsDirectory,
    DirectoryNotEmpty,
    ReadOnly,
    InvalidUtf8,
    VersionConflict { expected: u64, actual: u64 },
    LimitExceeded(&'static str),
    CounterExhausted,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resource store: {self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// Case normalization is fixed for the store's lifetime.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CasePolicy {
    #[default]
    Sensitive,
    /// Folds ASCII letters only; no Unicode case folding or normalization.
    AsciiInsensitive,
}

/// Retained logical limits, excluding the permanent root and snapshot retention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Files and directories, excluding root.
    pub max_entries: usize,
    pub max_total_bytes: usize,
    pub max_resource_bytes: usize,
    pub max_path_bytes: usize,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_total_bytes: 64 * 1024 * 1024,
            max_resource_bytes: 16 * 1024 * 1024,
            max_path_bytes: 4096,
            max_depth: 128,
        }
    }
}

/// Owner-controlled tags; these are not an authorization boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Attributes {
    pub hidden: bool,
    pub read_only: bool,
}
impl Attributes {
    fn inherit(self, other: Self) -> Self {
        Self {
            hidden: self.hidden || other.hidden,
            read_only: self.read_only || other.read_only,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    File,
    Directory,
}

/// Store-local stable identity. Moves preserve it; copies and recreation allocate new IDs.
/// IDs must be qualified by their owning store instance by the consumer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EntryId(u64);
impl EntryId {
    pub fn value(self) -> u64 {
        self.0
    }
}

/// Owned metadata. Versions describe direct mutations, not descendant changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub id: EntryId,
    pub path: String,
    pub kind: EntryKind,
    pub size: usize,
    pub attributes: Attributes,
    pub effective_attributes: Attributes,
    pub version: u64,
}

/// File creation and replacement intent must be explicit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteMode {
    Create,
    Replace,
    Upsert,
    /// Replace only if the existing file still has this version.
    CompareVersion(u64),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ListOptions {
    pub recursive: bool,
    /// Hidden entries and descendants of hidden folders are omitted by default.
    pub include_hidden: bool,
}

#[derive(Clone)]
struct Node {
    id: EntryId,
    bytes: Option<Arc<[u8]>>,
    attributes: Attributes,
    version: u64,
}
impl Node {
    fn directory(id: EntryId, version: u64) -> Self {
        Self {
            id,
            bytes: None,
            attributes: Attributes::default(),
            version,
        }
    }
}

/// Coherent immutable view. Clones share bytes but own their metadata indexes.
#[derive(Clone)]
pub struct Snapshot {
    nodes: BTreeMap<String, Node>,
    policy: CasePolicy,
    limits: Limits,
    generation: u64,
    next_id: u64,
}

impl Snapshot {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn case_policy(&self) -> CasePolicy {
        self.policy
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// File count, excluding directories.
    pub fn len(&self) -> usize {
        self.nodes.values().filter(|n| n.bytes.is_some()).count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Logical payload bytes; copies count separately even when sharing allocation.
    pub fn total_bytes(&self) -> usize {
        self.nodes
            .values()
            .filter_map(|n| n.bytes.as_ref())
            .map(|b| b.len())
            .sum()
    }
    pub fn normalize_path(&self, path: &str) -> Result<String> {
        path::normalize(path, self.policy, self.limits)
    }
    fn effective(&self, path: &str) -> Attributes {
        let mut attributes = Attributes::default();
        let mut current = path;
        loop {
            if let Some(node) = self.nodes.get(current) {
                attributes = attributes.inherit(node.attributes);
            }
            if current.is_empty() {
                break;
            }
            current = parent(current);
        }
        attributes
    }
    fn describe(&self, path: &str, node: &Node) -> Entry {
        Entry {
            id: node.id,
            path: path.to_owned(),
            kind: if node.bytes.is_some() {
                EntryKind::File
            } else {
                EntryKind::Directory
            },
            size: node.bytes.as_ref().map_or(0, |b| b.len()),
            attributes: node.attributes,
            effective_attributes: self.effective(path),
            version: node.version,
        }
    }
    /// Looks up either a file or directory, including hidden entries.
    pub fn entry(&self, path: &str) -> Result<Entry> {
        let path = self.normalize_path(path)?;
        let node = self.nodes.get(&path).ok_or(Error::NotFound)?;
        Ok(self.describe(&path, node))
    }
    /// Finds an entry after a move without retaining its old address.
    pub fn entry_by_id(&self, id: EntryId) -> Result<Entry> {
        self.nodes
            .iter()
            .find(|(_, n)| n.id == id)
            .map(|(p, n)| self.describe(p, n))
            .ok_or(Error::NotFound)
    }
    pub fn contains(&self, path: &str) -> Result<bool> {
        Ok(self.nodes.contains_key(&self.normalize_path(path)?))
    }
    /// Immutable bytes remain valid after overwrite, removal, or store drop.
    pub fn read(&self, path: &str) -> Result<Arc<[u8]>> {
        let path = self.normalize_path(path)?;
        self.nodes
            .get(&path)
            .ok_or(Error::NotFound)?
            .bytes
            .clone()
            .ok_or(Error::IsDirectory)
    }
    /// Strict UTF-8 without BOM removal or newline conversion.
    pub fn read_text(&self, path: &str) -> Result<String> {
        String::from_utf8(self.read(path)?.to_vec()).map_err(|_| Error::InvalidUtf8)
    }
    pub fn open(&self, path: &str) -> Result<Cursor<Arc<[u8]>>> {
        Ok(Cursor::new(self.read(path)?))
    }
    pub fn content_equals(&self, left: &str, right: &str) -> Result<bool> {
        Ok(self.read(left)? == self.read(right)?)
    }
    /// Deterministic lexical ordering by canonical path; excludes the directory itself.
    pub fn list(&self, directory: &str, options: ListOptions) -> Result<Vec<Entry>> {
        let directory = self.normalize_path(directory)?;
        let node = self.nodes.get(&directory).ok_or(Error::NotFound)?;
        if node.bytes.is_some() {
            return Err(Error::NotDirectory);
        }
        Ok(self
            .nodes
            .iter()
            .filter(|(p, _)| below(p, &directory) && (options.recursive || parent(p) == directory))
            .filter(|(p, _)| options.include_hidden || !self.effective(p).hidden)
            .map(|(p, n)| self.describe(p, n))
            .collect())
    }
    /// Resolves a relative reference against a file's parent without leaving root.
    /// Neither the base nor result must exist. URI syntax and percent escapes are rejected.
    pub fn resolve(&self, base_file: &str, reference: &str) -> Result<String> {
        let base = self.normalize_path(base_file)?;
        if base.is_empty() || reference.is_empty() || reference.starts_with('/') {
            return Err(Error::InvalidPath);
        }
        if reference.len() > self.limits.max_path_bytes {
            return Err(Error::LimitExceeded("path bytes"));
        }
        let mut parts: Vec<&str> = if parent(&base).is_empty() {
            vec![]
        } else {
            parent(&base).split('/').collect()
        };
        for part in reference.split('/') {
            match part {
                "" => return Err(Error::InvalidPath),
                "." => {}
                ".." => {
                    parts.pop().ok_or(Error::InvalidPath)?;
                }
                part => {
                    self.normalize_path(part)?;
                    parts.push(part);
                }
            }
        }
        self.normalize_path(&parts.join("/"))
    }
}

/// Single-owner mutable store. Read APIs are inherited from [Snapshot].
/// Use an application-owned RwLock when sharing mutations across threads.
pub struct ResourceStore {
    view: Snapshot,
}
impl Deref for ResourceStore {
    type Target = Snapshot;
    fn deref(&self) -> &Snapshot {
        &self.view
    }
}
impl Default for ResourceStore {
    fn default() -> Self {
        Self::new(CasePolicy::default(), Limits::default())
    }
}
impl ResourceStore {
    pub fn new(policy: CasePolicy, limits: Limits) -> Self {
        Self {
            view: Snapshot {
                nodes: BTreeMap::from([(String::new(), Node::directory(EntryId(0), 0))]),
                policy,
                limits,
                generation: 0,
                next_id: 1,
            },
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.view.clone()
    }

    // Stage metadata and shared bytes so every ordinary Result failure is atomic.
    fn transaction<T>(&mut self, operation: impl FnOnce(&mut Snapshot) -> Result<T>) -> Result<T> {
        let mut next = self.view.clone();
        next.generation = next
            .generation
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        let result = operation(&mut next)?;
        next.validate_limits()?;
        self.view = next;
        Ok(result)
    }

    /// Creates missing ancestors; an existing directory is an idempotent success.
    pub fn create_directory(&mut self, path: &str) -> Result<Entry> {
        let path = self.normalize_path(path)?;
        if let Some(node) = self.nodes.get(&path) {
            return if node.bytes.is_none() {
                self.entry(&path)
            } else {
                Err(Error::AlreadyExists)
            };
        }
        self.transaction(|s| {
            s.writable(&path)?;
            s.ensure_parents(&path)?;
            let id = s.allocate_id()?;
            s.nodes
                .insert(path.clone(), Node::directory(id, s.generation));
            s.entry(&path)
        })
    }
    pub fn write(&mut self, path: &str, bytes: impl AsRef<[u8]>, mode: WriteMode) -> Result<Entry> {
        let path = self.normalize_path(path)?;
        let bytes = bytes.as_ref();
        if bytes.len() > self.limits.max_resource_bytes {
            return Err(Error::LimitExceeded("resource bytes"));
        }
        self.transaction(|s| s.write_inner(&path, bytes, mode))
    }
    pub fn write_text(&mut self, path: &str, text: &str, mode: WriteMode) -> Result<Entry> {
        self.write(path, text.as_bytes(), mode)
    }
    /// Appends strict UTF-8, creating a missing file. Existing invalid UTF-8 fails unchanged.
    pub fn append_text(&mut self, path: &str, suffix: &str) -> Result<Entry> {
        let mut text = match self.read_text(path) {
            Ok(text) => text,
            Err(Error::NotFound) => String::new(),
            Err(e) => return Err(e),
        };
        let size = text
            .len()
            .checked_add(suffix.len())
            .ok_or(Error::LimitExceeded("resource bytes"))?;
        if size > self.limits.max_resource_bytes {
            return Err(Error::LimitExceeded("resource bytes"));
        }
        text.push_str(suffix);
        self.write_text(path, &text, WriteMode::Upsert)
    }
    /// Applies all writes in order in one transaction; any failure rolls back all of them.
    pub fn write_many<'a>(
        &mut self,
        items: impl IntoIterator<Item = (&'a str, &'a [u8])>,
        mode: WriteMode,
    ) -> Result<()> {
        self.transaction(|s| {
            for (path, bytes) in items {
                let path = s.normalize_path(path)?;
                s.write_inner(&path, bytes, mode)?;
                s.validate_limits()?;
            }
            Ok(())
        })
    }
    /// An owner may clear its own read-only tag, but not through a read-only ancestor.
    pub fn set_attributes(&mut self, path: &str, attributes: Attributes) -> Result<Entry> {
        let path = self.normalize_path(path)?;
        self.transaction(|s| {
            if !path.is_empty() {
                s.writable(parent(&path))?;
            }
            let node = s.nodes.get_mut(&path).ok_or(Error::NotFound)?;
            node.attributes = attributes;
            node.version = s.generation;
            s.entry(&path)
        })
    }
    /// Removes a file or directory; recursive deletion preflights all descendants.
    /// Missing paths return false. The permanent root cannot be removed.
    pub fn remove(&mut self, path: &str, recursive: bool) -> Result<bool> {
        let path = self.normalize_path(path)?;
        if path.is_empty() {
            return Err(Error::InvalidPath);
        }
        if !self.nodes.contains_key(&path) {
            return Ok(false);
        }
        self.transaction(|s| {
            let keys: Vec<_> = s
                .nodes
                .keys()
                .filter(|p| **p == path || below(p, &path))
                .cloned()
                .collect();
            if !recursive && keys.len() > 1 {
                return Err(Error::DirectoryNotEmpty);
            }
            for key in &keys {
                s.writable(key)?;
            }
            for key in keys {
                s.nodes.remove(&key);
            }
            Ok(true)
        })
    }
    /// Removes all children while preserving root and its attributes.
    pub fn clear(&mut self) -> Result<()> {
        self.transaction(|s| {
            for path in s.nodes.keys() {
                s.writable(path)?;
            }
            s.nodes.retain(|p, _| p.is_empty());
            Ok(())
        })
    }
    /// Copies a file or entire subtree to a new address. Never overwrites or merges.
    pub fn copy(&mut self, source: &str, destination: &str) -> Result<Entry> {
        self.transfer(source, destination, false)
    }
    /// Moves a file or entire subtree atomically. Never overwrites or merges.
    pub fn move_entry(&mut self, source: &str, destination: &str) -> Result<Entry> {
        self.transfer(source, destination, true)
    }
    fn transfer(&mut self, source: &str, destination: &str, moving: bool) -> Result<Entry> {
        let source = self.normalize_path(source)?;
        let destination = self.normalize_path(destination)?;
        if source.is_empty()
            || destination.is_empty()
            || source == destination
            || below(&destination, &source)
        {
            return Err(Error::InvalidPath);
        }
        self.transaction(|s| {
            if !s.nodes.contains_key(&source) {
                return Err(Error::NotFound);
            }
            if s.nodes.contains_key(&destination) {
                return Err(Error::AlreadyExists);
            }
            s.writable(&destination)?;
            s.ensure_parents(&destination)?;
            let items: Vec<_> = s
                .nodes
                .iter()
                .filter(|(p, _)| **p == source || below(p, &source))
                .map(|(p, n)| (p.clone(), n.clone()))
                .collect();
            for (path, _) in &items {
                if moving {
                    s.writable(path)?;
                }
            }
            for (path, mut node) in items {
                let target = format!("{destination}{}", &path[source.len()..]);
                s.normalize_path(&target)?;
                if !moving {
                    node.id = s.allocate_id()?;
                }
                node.version = s.generation;
                if moving {
                    s.nodes.remove(&path);
                }
                s.nodes.insert(target, node);
            }
            s.entry(&destination)
        })
    }
}

impl Snapshot {
    fn allocate_id(&mut self) -> Result<EntryId> {
        let id = EntryId(self.next_id);
        self.next_id = self.next_id.checked_add(1).ok_or(Error::CounterExhausted)?;
        Ok(id)
    }
    fn writable(&self, path: &str) -> Result<()> {
        if self.effective(path).read_only {
            Err(Error::ReadOnly)
        } else {
            Ok(())
        }
    }
    fn ensure_parents(&mut self, path: &str) -> Result<()> {
        let mut current = parent(path);
        while !current.is_empty() {
            match self.nodes.get(current) {
                Some(n) if n.bytes.is_some() => return Err(Error::NotDirectory),
                Some(_) => {}
                None => {
                    let id = self.allocate_id()?;
                    self.nodes
                        .insert(current.to_owned(), Node::directory(id, self.generation));
                }
            }
            current = parent(current);
        }
        Ok(())
    }
    fn write_inner(&mut self, path: &str, bytes: &[u8], mode: WriteMode) -> Result<Entry> {
        self.writable(path)?;
        let existing = self.nodes.get(path);
        if existing.is_some_and(|n| n.bytes.is_none()) {
            return Err(Error::IsDirectory);
        }
        match (mode, existing) {
            (WriteMode::Create, Some(_)) => return Err(Error::AlreadyExists),
            (WriteMode::Replace | WriteMode::CompareVersion(_), None) => {
                return Err(Error::NotFound);
            }
            (WriteMode::CompareVersion(expected), Some(node)) if node.version != expected => {
                return Err(Error::VersionConflict {
                    expected,
                    actual: node.version,
                });
            }
            _ => {}
        }
        if bytes.len() > self.limits.max_resource_bytes {
            return Err(Error::LimitExceeded("resource bytes"));
        }
        let old_size = existing
            .and_then(|n| n.bytes.as_ref())
            .map_or(0, |b| b.len());
        let size = self
            .total_bytes()
            .checked_sub(old_size)
            .and_then(|n| n.checked_add(bytes.len()))
            .ok_or(Error::LimitExceeded("total bytes"))?;
        if size > self.limits.max_total_bytes {
            return Err(Error::LimitExceeded("total bytes"));
        }
        let attributes = existing.map_or(Attributes::default(), |n| n.attributes);
        let id = match existing {
            Some(n) => n.id,
            None => self.allocate_id()?,
        };
        self.ensure_parents(path)?;
        self.nodes.insert(
            path.to_owned(),
            Node {
                id,
                bytes: Some(Arc::from(bytes)),
                attributes,
                version: self.generation,
            },
        );
        self.entry(path)
    }
    fn validate_limits(&self) -> Result<()> {
        if self.nodes.len() - 1 > self.limits.max_entries {
            return Err(Error::LimitExceeded("entries"));
        }
        let total = self
            .nodes
            .values()
            .filter_map(|n| n.bytes.as_ref())
            .try_fold(0usize, |sum, bytes| sum.checked_add(bytes.len()))
            .ok_or(Error::LimitExceeded("total bytes"))?;
        if total > self.limits.max_total_bytes {
            return Err(Error::LimitExceeded("total bytes"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod counter_tests {
    use super::*;

    #[test]
    fn exhausted_generation_fails_without_mutation() {
        let mut store = ResourceStore::default();
        store.view.generation = u64::MAX;
        assert_eq!(
            store.write("a", [], WriteMode::Create),
            Err(Error::CounterExhausted)
        );
        assert_eq!(store.generation(), u64::MAX);
        assert!(!store.contains("a").unwrap());
    }

    #[test]
    fn exhausted_identity_rolls_back_parent_creation() {
        let mut store = ResourceStore::default();
        store.view.next_id = u64::MAX - 1;
        assert_eq!(store.create_directory("a/b"), Err(Error::CounterExhausted));
        assert!(!store.contains("a").unwrap());
        assert_eq!(store.view.next_id, u64::MAX - 1);
        assert_eq!(store.generation(), 0);
    }
}
