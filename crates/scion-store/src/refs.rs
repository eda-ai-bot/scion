//! Ref policy. Server-side refs are deliberately scarce: global state that every
//! server must agree on is a scaling liability, so only `main` and `release/*`
//! exist. Everything else is an anonymous head tracked client-side.

use scion_types::ChangesetId;

/// The current target and CAS version of a ref.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefState {
    pub target: ChangesetId,
    /// Incremented on every successful move. `move_ref` requires the caller to
    /// present the version they last observed.
    pub version: u64,
}

/// Whether a ref name may exist on the server.
#[must_use]
pub fn is_allowed_ref_name(name: &str) -> bool {
    if name == "main" {
        return true;
    }
    match name.strip_prefix("release/") {
        Some(rest) => {
            !rest.is_empty()
                && !rest.contains("//")
                && !rest.ends_with('/')
                && rest
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy() {
        assert!(is_allowed_ref_name("main"));
        assert!(is_allowed_ref_name("release/1.2"));
        assert!(is_allowed_ref_name("release/2026/q4"));
        assert!(!is_allowed_ref_name("master"));
        assert!(!is_allowed_ref_name("feature/x"));
        assert!(!is_allowed_ref_name("release/"));
        assert!(!is_allowed_ref_name("release/a//b"));
        assert!(!is_allowed_ref_name("release/a b"));
    }
}
