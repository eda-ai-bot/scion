//! Trees: directory listings, derived from changesets and stored in the object
//! store under `trees/{id}`.

use std::collections::BTreeMap;

use crate::canonical::Encode;
use crate::{BlobId, FileType, PathComponent, TreeId};

/// One entry in a tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TreeEntry {
    /// A subdirectory.
    Tree { id: TreeId },
    /// A file.
    Blob {
        id: BlobId,
        kind: FileType,
        size: u64,
    },
}

/// A directory listing, sorted by component name (byte order).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tree {
    pub entries: BTreeMap<PathComponent, TreeEntry>,
}

impl Tree {
    /// The empty tree.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Compute the content-addressed id: `blake3(canonical(self))`.
    #[must_use]
    pub fn id(&self) -> TreeId {
        TreeId::hash(&self.encode_to_vec())
    }

    /// Whether the tree has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
