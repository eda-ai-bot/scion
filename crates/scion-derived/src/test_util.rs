//! Shared test helpers: naive materialisation, changeset builders, a counting
//! object store, and proptest generators.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use bytes::Bytes;
use proptest::prelude::*;
use scion_store::{MemoryStore, ObjectStore};
use scion_types::{
    BlobId, Changeset, ChangesetId, FileChange, FileType, Identity, RepoPath, Timestamp, Tree,
    TreeEntry, TreeId,
};

pub const REPO: &str = "t";

/// `path -> file`: the naive, fully materialised view of a tree.
pub type Flat = BTreeMap<RepoPath, (BlobId, FileType, u64)>;

/// Flatten a tree into `path -> file`.
pub async fn materialize(objects: &dyn ObjectStore, root: TreeId) -> Flat {
    let mut out = Flat::new();
    let mut stack = vec![(None::<RepoPath>, root)];
    while let Some((prefix, id)) = stack.pop() {
        let tree = objects.get_tree(id).await.unwrap().expect("tree exists");
        for (name, entry) in tree.entries {
            let path = match &prefix {
                Some(p) => p.join(&name),
                None => RepoPath::new(name.as_str()).unwrap(),
            };
            match entry {
                TreeEntry::Blob { id, kind, size } => {
                    out.insert(path, (id, kind, size));
                }
                TreeEntry::Tree { id } => stack.push((Some(path), id)),
            }
        }
    }
    out
}

pub fn changeset(
    parents: Vec<ChangesetId>,
    changes: BTreeMap<RepoPath, FileChange>,
    msg: &str,
) -> Changeset {
    Changeset {
        repo: REPO.into(),
        parents,
        author: Identity::new("a", "a@x"),
        author_date: Timestamp::new(0, 0),
        committer: None,
        committer_date: None,
        message: msg.into(),
        file_changes: changes,
        extra: BTreeMap::new(),
    }
}

pub fn tracked(content: &[u8]) -> FileChange {
    FileChange::Tracked {
        content: BlobId::hash(content),
        kind: FileType::Regular,
        size: u64::try_from(content.len()).unwrap(),
        copy_from: None,
    }
}

/// Counts writes so tests can bound the number of rewritten nodes.
pub struct Counting<'a> {
    pub inner: &'a MemoryStore,
    pub tree_puts: AtomicUsize,
    pub blob_puts: AtomicUsize,
}

impl<'a> Counting<'a> {
    pub fn new(inner: &'a MemoryStore) -> Self {
        Self {
            inner,
            tree_puts: AtomicUsize::new(0),
            blob_puts: AtomicUsize::new(0),
        }
    }
    pub fn tree_puts(&self) -> usize {
        self.tree_puts.load(Ordering::Relaxed)
    }
    pub fn blob_puts(&self) -> usize {
        self.blob_puts.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl ObjectStore for Counting<'_> {
    async fn put_blob(&self, bytes: Bytes) -> scion_store::Result<BlobId> {
        self.blob_puts.fetch_add(1, Ordering::Relaxed);
        self.inner.put_blob(bytes).await
    }
    async fn get_blob(&self, id: BlobId) -> scion_store::Result<Option<Bytes>> {
        self.inner.get_blob(id).await
    }
    async fn has_blobs(&self, ids: &[BlobId]) -> scion_store::Result<Vec<bool>> {
        self.inner.has_blobs(ids).await
    }
    async fn put_tree(&self, tree: &Tree) -> scion_store::Result<TreeId> {
        self.tree_puts.fetch_add(1, Ordering::Relaxed);
        self.inner.put_tree(tree).await
    }
    async fn get_tree(&self, id: TreeId) -> scion_store::Result<Option<Tree>> {
        self.inner.get_tree(id).await
    }
}

/// Small alphabet so paths collide and nest (file/dir replacement happens).
pub fn arb_path() -> impl Strategy<Value = RepoPath> {
    prop::collection::vec("[ab][0-2]", 1..4).prop_map(|cs| RepoPath::new(cs.join("/")).unwrap())
}

pub fn arb_change() -> impl Strategy<Value = FileChange> {
    prop_oneof![
        2 => any::<u8>().prop_map(|b| tracked(&[b])),
        1 => Just(FileChange::Deleted),
    ]
}

pub fn arb_changes() -> impl Strategy<Value = BTreeMap<RepoPath, FileChange>> {
    prop::collection::btree_map(arb_path(), arb_change(), 0..6)
}
