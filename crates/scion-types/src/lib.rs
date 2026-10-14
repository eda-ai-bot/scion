//! Core data model for Scion.
//!
//! This crate defines the *inherent* data of a repository — the things that are
//! content-addressed and form the Merkle DAG:
//!
//! * [`Changeset`]: a flat list of files changed relative to the first parent,
//!   plus commit metadata. Its [`ChangesetId`] is `blake3(canonical(changeset))`.
//! * [`Tree`]: a directory listing. Trees are *derived* from changesets by the
//!   server, never stored inside a changeset. `TreeId = blake3(canonical(tree))`.
//! * Blobs: raw file bytes. `BlobId = blake3(bytes)`.
//!
//! The byte-level canonical encoding lives in [`canonical`] and is specified in
//! `docs/DATA_MODEL.md`. It is deliberately *not* protobuf: hash stability must
//! never depend on wire-format evolution.
//!
//! Nothing in this crate performs I/O.

#![forbid(unsafe_code)]

pub mod canonical;
mod changeset;
mod id;
mod path;
mod tree;

pub use canonical::{CanonicalError, Decode, Encode};
pub use changeset::{Changeset, FileChange, FileType, Identity, Timestamp};
pub use id::{BlobId, ChangesetId, IdParseError, TreeId};
pub use path::{PathComponent, PathError, RepoPath};
pub use tree::{Tree, TreeEntry};
