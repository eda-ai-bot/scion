//! Three-way line merge.
//!
//! [`merge`] combines two versions of a file (`ours`, `theirs`) that diverged
//! from a common `base`. Changes that touch different regions are both
//! applied; changes that overlap are rendered as a conflict block in
//! `diff3` style (ours, base, theirs between markers) so that the reader —
//! human or agent — sees every side.
//!
//! The implementation is `diffy`'s merge; this crate fixes the options, adds
//! the trivial fast paths, and reports the number of conflict blocks. Files
//! are byte strings split on `\n`; non-UTF-8 content is fine.

#![forbid(unsafe_code)]

use diffy::{ConflictStyle, MergeOptions};

/// Length of the conflict marker lines (`<<<<<<<`, `|||||||`, `=======`, `>>>>>>>`).
pub const MARKER_LEN: usize = 7;

/// Outcome of a three-way merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// Every change applied without overlap.
    Clean(Vec<u8>),
    /// At least one overlap; `content` contains conflict blocks.
    Conflicted { content: Vec<u8>, conflicts: usize },
}

impl Merged {
    /// Whether the merge produced no conflict.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Clean(_))
    }

    /// The merged bytes, with conflict markers if any.
    #[must_use]
    pub fn into_content(self) -> Vec<u8> {
        match self {
            Self::Clean(c) | Self::Conflicted { content: c, .. } => c,
        }
    }
}

/// Merge `ours` and `theirs`, both derived from `base`.
#[must_use]
pub fn merge(base: &[u8], ours: &[u8], theirs: &[u8]) -> Merged {
    // Trivial cases need no diff.
    if ours == theirs || theirs == base {
        return Merged::Clean(ours.to_vec());
    }
    if ours == base {
        return Merged::Clean(theirs.to_vec());
    }
    let mut options = MergeOptions::new();
    options
        .set_conflict_style(ConflictStyle::Diff3)
        .set_conflict_marker_length(MARKER_LEN);
    match options.merge_bytes(base, ours, theirs) {
        Ok(content) => Merged::Clean(content),
        Err(content) => {
            let conflicts = count_conflicts(&content);
            Merged::Conflicted { content, conflicts }
        }
    }
}

/// Number of conflict blocks: lines that start with the opening marker.
fn count_conflicts(content: &[u8]) -> usize {
    let opener = [b'<'; MARKER_LEN];
    content
        .split(|b| *b == b'\n')
        .filter(|line| line.starts_with(&opener))
        .count()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn text(lines: &[u32]) -> Vec<u8> {
        let mut out = Vec::new();
        for l in lines {
            out.extend_from_slice(format!("line-{l}\n").as_bytes());
        }
        out
    }

    #[test]
    fn trivial_cases() {
        let (b, o, t) = (
            b"a\nb\n".as_slice(),
            b"a\nX\n".as_slice(),
            b"a\nb\nc\n".as_slice(),
        );
        assert_eq!(merge(b, b, b), Merged::Clean(b.to_vec()));
        assert_eq!(merge(b, o, b), Merged::Clean(o.to_vec()));
        assert_eq!(merge(b, b, t), Merged::Clean(t.to_vec()));
        assert_eq!(merge(b, o, o), Merged::Clean(o.to_vec()));
    }

    #[test]
    fn conflict_block_shows_all_three_sides() {
        let base = b"a\nb\nc\n";
        let ours = b"a\nB1\nc\n";
        let theirs = b"a\nB2\nc\n";
        let Merged::Conflicted { content, conflicts } = merge(base, ours, theirs) else {
            panic!("expected a conflict");
        };
        assert_eq!(conflicts, 1);
        let s = String::from_utf8(content).unwrap();
        assert_eq!(
            s,
            "a\n<<<<<<< ours\nB1\n||||||| original\nb\n=======\nB2\n>>>>>>> theirs\nc\n"
        );
    }

    #[test]
    fn no_trailing_newline_and_non_utf8() {
        let base = b"\xff\nmid\nend";
        let ours = b"\xff\nmid\nend\nmore";
        let theirs = b"\xfe\nmid\nend";
        assert_eq!(
            merge(base, ours, theirs),
            Merged::Clean(b"\xfe\nmid\nend\nmore".to_vec())
        );
    }

    // ---- property tests ---------------------------------------------------

    /// base = p0 ++ a ++ p1 ++ b ++ p2 with unique lines and |p1| >= 2, so a
    /// change to `a` and a change to `b` are non-overlapping by construction.
    #[derive(Debug, Clone)]
    struct Split {
        p0: Vec<u32>,
        a: Vec<u32>,
        p1: Vec<u32>,
        b: Vec<u32>,
        p2: Vec<u32>,
    }

    fn arb_split() -> impl Strategy<Value = Split> {
        (0usize..3, 0usize..3, 2usize..4, 0usize..3, 0usize..3).prop_map(|(n0, na, n1, nb, n2)| {
            let mut next = 0u32;
            let mut take = |n: usize| {
                let v: Vec<u32> = (next..next + u32::try_from(n).unwrap()).collect();
                next += u32::try_from(n).unwrap();
                v
            };
            Split {
                p0: take(n0),
                a: take(na),
                p1: take(n1),
                b: take(nb),
                p2: take(n2),
            }
        })
    }

    /// Fresh lines that appear in no other region.
    fn fresh(start: u32, n: usize) -> Vec<u32> {
        (start..start + u32::try_from(n).unwrap()).collect()
    }

    fn concat(parts: &[&[u32]]) -> Vec<u32> {
        parts.iter().flat_map(|p| p.iter().copied()).collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Non-overlapping edits merge cleanly to exactly "both edits applied".
        #[test]
        fn disjoint_edits_merge_cleanly(s in arb_split(), na in 0usize..3, nb in 0usize..3) {
            let a2 = fresh(1000, na);
            let b2 = fresh(2000, nb);
            let base = concat(&[&s.p0, &s.a, &s.p1, &s.b, &s.p2]);
            let ours = concat(&[&s.p0, &a2, &s.p1, &s.b, &s.p2]);
            let theirs = concat(&[&s.p0, &s.a, &s.p1, &b2, &s.p2]);
            let expected = concat(&[&s.p0, &a2, &s.p1, &b2, &s.p2]);
            prop_assert_eq!(merge(&text(&base), &text(&ours), &text(&theirs)), Merged::Clean(text(&expected)));
            // Symmetric.
            prop_assert_eq!(merge(&text(&base), &text(&theirs), &text(&ours)), Merged::Clean(text(&expected)));
        }

        /// The same region edited differently on both sides is one conflict,
        /// and the conflict block carries both sides.
        #[test]
        fn overlapping_edits_conflict(s in arb_split(), na in 1usize..3, nb in 1usize..3) {
            let a_ours = fresh(1000, na);
            let a_theirs = fresh(2000, nb);
            let base = concat(&[&s.p0, &s.a, &s.p1, &s.b, &s.p2]);
            let ours = concat(&[&s.p0, &a_ours, &s.p1, &s.b, &s.p2]);
            let theirs = concat(&[&s.p0, &a_theirs, &s.p1, &s.b, &s.p2]);
            let Merged::Conflicted { content, conflicts } = merge(&text(&base), &text(&ours), &text(&theirs)) else {
                return Err(TestCaseError::fail("expected a conflict"));
            };
            prop_assert_eq!(conflicts, 1);
            for l in a_ours.iter().chain(&a_theirs) {
                let needle = format!("line-{l}\n");
                prop_assert!(content.windows(needle.len()).any(|w| w == needle.as_bytes()), "missing {}", needle);
            }
            // Untouched context survives intact, in order.
            let context = text(&concat(&[&s.p1, &s.b, &s.p2]));
            prop_assert!(content.ends_with(&context));
            prop_assert!(content.starts_with(&text(&s.p0)));
        }
    }
}
