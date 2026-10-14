//! Golden-hash tests.
//!
//! Each fixture is a fully specified object. Its canonical bytes and id are
//! checked in under `tests/golden/<name>.{bin,hash}`. If the encoding changes,
//! these tests fail: that is the point. Bump `FORMAT_VERSION`, regenerate with
//! `UPDATE_GOLDEN=1 cargo test -p scion-types --test golden`, and explain the
//! change in `docs/DATA_MODEL.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use scion_types::{
    BlobId, Changeset, ChangesetId, Encode, FileChange, FileType, Identity, PathComponent,
    RepoPath, Timestamp, Tree, TreeEntry, TreeId,
};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn check(name: &str, bytes: &[u8], id_hex: &str) {
    let dir = golden_dir();
    let bin_path = dir.join(format!("{name}.bin"));
    let hash_path = dir.join(format!("{name}.hash"));

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&bin_path, bytes).unwrap();
        std::fs::write(&hash_path, format!("{id_hex}\n")).unwrap();
        return;
    }

    let expected_bin = std::fs::read(&bin_path).unwrap_or_else(|e| {
        panic!(
            "missing golden {}: {e} (run with UPDATE_GOLDEN=1)",
            bin_path.display()
        )
    });
    let expected_hash = std::fs::read_to_string(&hash_path).unwrap();

    assert_eq!(
        bytes,
        expected_bin.as_slice(),
        "canonical bytes changed for {name}"
    );
    assert_eq!(id_hex, expected_hash.trim(), "hash changed for {name}");
}

fn id32(fill: u8) -> [u8; 32] {
    [fill; 32]
}

#[test]
fn empty_tree() {
    let t = Tree::empty();
    check("empty_tree", &t.encode_to_vec(), &t.id().to_hex());
}

#[test]
fn small_tree() {
    let mut entries = BTreeMap::new();
    entries.insert(
        PathComponent::new("README.md").unwrap(),
        TreeEntry::Blob {
            id: BlobId::from_bytes(id32(0x11)),
            kind: FileType::Regular,
            size: 1234,
        },
    );
    entries.insert(
        PathComponent::new("build.sh").unwrap(),
        TreeEntry::Blob {
            id: BlobId::from_bytes(id32(0x22)),
            kind: FileType::Executable,
            size: 77,
        },
    );
    entries.insert(
        PathComponent::new("src").unwrap(),
        TreeEntry::Tree {
            id: TreeId::from_bytes(id32(0x33)),
        },
    );
    let t = Tree { entries };
    check("small_tree", &t.encode_to_vec(), &t.id().to_hex());
}

#[test]
fn root_changeset() {
    let mut file_changes = BTreeMap::new();
    file_changes.insert(
        RepoPath::new("README.md").unwrap(),
        FileChange::Tracked {
            content: BlobId::from_bytes(id32(0xaa)),
            kind: FileType::Regular,
            size: 42,
            copy_from: None,
        },
    );
    let cs = Changeset {
        repo: "scion".into(),
        parents: vec![],
        author: Identity::new("Ada Lovelace", "ada@example.com"),
        author_date: Timestamp::new(1_700_000_000, 0),
        committer: None,
        committer_date: None,
        message: "initial".into(),
        file_changes,
        extra: BTreeMap::new(),
    };
    check("root_changeset", &cs.encode_to_vec(), &cs.id().to_hex());
}

/// Mirrors the example changeset in the Scion article's "Shape of a changeset"
/// figure: one modified doc, one deleted source file, one executable, with a
/// separate committer and a git-sha extra.
#[test]
fn full_changeset() {
    let mut file_changes = BTreeMap::new();
    file_changes.insert(
        RepoPath::new("docs/guide.md").unwrap(),
        FileChange::Tracked {
            content: BlobId::from_bytes(id32(0x3e)),
            kind: FileType::Regular,
            size: 2048,
            copy_from: Some((
                RepoPath::new("docs/old-guide.md").unwrap(),
                ChangesetId::from_bytes(id32(0xd4)),
            )),
        },
    );
    file_changes.insert(RepoPath::new("src/legacy.rs").unwrap(), FileChange::Deleted);
    file_changes.insert(
        RepoPath::new("tools/build.sh").unwrap(),
        FileChange::Tracked {
            content: BlobId::from_bytes(id32(0x9b)),
            kind: FileType::Executable,
            size: 512,
            copy_from: None,
        },
    );
    let mut extra = BTreeMap::new();
    extra.insert("git-sha".to_owned(), vec![0xde, 0xad, 0xbe, 0xef]);

    let cs = Changeset {
        repo: "monorepo".into(),
        parents: vec![
            ChangesetId::from_bytes(id32(0xd4)),
            ChangesetId::from_bytes(id32(0x88)),
        ],
        author: Identity::new("Claude", "noreply@example.com"),
        author_date: Timestamp::new(1_789_435_509, 0),
        committer: Some(Identity::new("Scion", "noreply+scion@example.com")),
        committer_date: Some(Timestamp::new(1_789_435_900, 180)),
        message: "scion-cache: serve blobs from the memory tier".into(),
        file_changes,
        extra,
    };
    check("full_changeset", &cs.encode_to_vec(), &cs.id().to_hex());
}
