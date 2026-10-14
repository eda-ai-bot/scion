//! Property tests: `decode(encode(x)) == x`, distinct values hash differently,
//! and the encoding is injective on the generated domain.

use std::collections::BTreeMap;

use proptest::prelude::*;
use scion_types::{
    BlobId, Changeset, ChangesetId, Decode, Encode, FileChange, FileType, Identity, PathComponent,
    RepoPath, Timestamp, Tree, TreeEntry, TreeId,
};

fn arb_id() -> impl Strategy<Value = [u8; 32]> {
    any::<[u8; 32]>()
}

fn arb_component() -> impl Strategy<Value = PathComponent> {
    // Printable, no '/', not "." or "..", non-empty.
    "[a-zA-Z0-9_.-]{1,12}"
        .prop_filter("reserved", |s| s != "." && s != "..")
        .prop_map(|s| PathComponent::new(s).unwrap())
}

fn arb_path() -> impl Strategy<Value = RepoPath> {
    prop::collection::vec(arb_component(), 1..4).prop_map(|cs| {
        let s = cs
            .iter()
            .map(PathComponent::as_str)
            .collect::<Vec<_>>()
            .join("/");
        RepoPath::new(s).unwrap()
    })
}

fn arb_file_type() -> impl Strategy<Value = FileType> {
    prop_oneof![
        Just(FileType::Regular),
        Just(FileType::Executable),
        Just(FileType::Symlink)
    ]
}

fn arb_identity() -> impl Strategy<Value = Identity> {
    (".{0,20}", ".{0,20}").prop_map(|(n, e)| Identity::new(n, e))
}

fn arb_timestamp() -> impl Strategy<Value = Timestamp> {
    (any::<i64>(), -1440i32..=1440).prop_map(|(s, tz)| Timestamp::new(s, tz))
}

fn arb_file_change() -> impl Strategy<Value = FileChange> {
    prop_oneof![
        Just(FileChange::Deleted),
        (
            arb_id(),
            arb_file_type(),
            any::<u64>(),
            prop::option::of((arb_path(), arb_id()))
        )
            .prop_map(|(c, kind, size, cf)| FileChange::Tracked {
                content: BlobId::from_bytes(c),
                kind,
                size,
                copy_from: cf.map(|(p, cs)| (p, ChangesetId::from_bytes(cs))),
            }),
    ]
}

fn arb_changeset() -> impl Strategy<Value = Changeset> {
    (
        "[a-z]{1,8}",
        prop::collection::vec(arb_id(), 0..3),
        arb_identity(),
        arb_timestamp(),
        prop::option::of(arb_identity()),
        prop::option::of(arb_timestamp()),
        ".{0,64}",
        prop::collection::btree_map(arb_path(), arb_file_change(), 0..6),
        prop::collection::btree_map(
            "[a-z-]{1,8}",
            prop::collection::vec(any::<u8>(), 0..8),
            0..3,
        ),
    )
        .prop_map(
            |(
                repo,
                parents,
                author,
                author_date,
                committer,
                committer_date,
                message,
                file_changes,
                extra,
            )| {
                Changeset {
                    repo,
                    parents: parents.into_iter().map(ChangesetId::from_bytes).collect(),
                    author,
                    author_date,
                    committer,
                    committer_date,
                    message,
                    file_changes,
                    extra,
                }
            },
        )
}

fn arb_tree() -> impl Strategy<Value = Tree> {
    prop::collection::btree_map(
        arb_component(),
        prop_oneof![
            arb_id().prop_map(|id| TreeEntry::Tree {
                id: TreeId::from_bytes(id)
            }),
            (arb_id(), arb_file_type(), any::<u64>()).prop_map(|(id, kind, size)| {
                TreeEntry::Blob {
                    id: BlobId::from_bytes(id),
                    kind,
                    size,
                }
            }),
        ],
        0..8,
    )
    .prop_map(|entries| Tree { entries })
}

proptest! {
    #[test]
    fn changeset_roundtrip(cs in arb_changeset()) {
        let bytes = cs.encode_to_vec();
        let decoded = Changeset::decode(&bytes).unwrap();
        prop_assert_eq!(&decoded, &cs);
        prop_assert_eq!(decoded.id(), cs.id());
    }

    #[test]
    fn tree_roundtrip(t in arb_tree()) {
        let bytes = t.encode_to_vec();
        let decoded = Tree::decode(&bytes).unwrap();
        prop_assert_eq!(&decoded, &t);
        prop_assert_eq!(decoded.id(), t.id());
    }

    #[test]
    fn distinct_changesets_hash_differently(a in arb_changeset(), b in arb_changeset()) {
        prop_assume!(a != b);
        prop_assert_ne!(a.id(), b.id());
    }

    #[test]
    fn distinct_trees_hash_differently(a in arb_tree(), b in arb_tree()) {
        prop_assume!(a != b);
        prop_assert_ne!(a.id(), b.id());
    }

    #[test]
    fn any_single_byte_flip_is_rejected_or_changes_value(t in arb_tree(), idx in any::<prop::sample::Index>(), flip in 1u8..=255) {
        let bytes = t.encode_to_vec();
        let i = idx.index(bytes.len());
        let mut mutated = bytes.clone();
        mutated[i] ^= flip;
        if let Ok(other) = Tree::decode(&mutated) {
            prop_assert_ne!(other, t);
        }
    }
}

#[test]
fn message_and_extra_are_not_confusable() {
    // Moving bytes between adjacent length-prefixed fields must change the hash.
    let base = Changeset {
        repo: "r".into(),
        parents: vec![],
        author: Identity::new("a", "b"),
        author_date: Timestamp::new(0, 0),
        committer: None,
        committer_date: None,
        message: "ab".into(),
        file_changes: BTreeMap::new(),
        extra: BTreeMap::new(),
    };
    let mut other = base.clone();
    other.message = "a".into();
    other.extra.insert("b".into(), vec![]);
    assert_ne!(base.id(), other.id());
}
