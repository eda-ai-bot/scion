//! Repository paths.
//!
//! Paths are byte-exact: no Unicode normalisation, no case folding. Ordering is
//! plain byte-lexicographic ordering of the full path string, which is what the
//! canonical encoding relies on.

use std::fmt;

/// Error returned when a path or component is malformed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("path is empty")]
    Empty,
    #[error("path has a leading or trailing '/'")]
    LeadingOrTrailingSlash,
    #[error("path has an empty component ('//')")]
    EmptyComponent,
    #[error("path component '{0}' is not allowed")]
    ReservedComponent(String),
    #[error("path contains a NUL byte")]
    Nul,
    #[error("component contains '/'")]
    SlashInComponent,
}

fn check_component(c: &str) -> Result<(), PathError> {
    if c.is_empty() {
        return Err(PathError::EmptyComponent);
    }
    if c == "." || c == ".." {
        return Err(PathError::ReservedComponent(c.to_owned()));
    }
    if c.contains('\0') {
        return Err(PathError::Nul);
    }
    Ok(())
}

/// A full path from the repository root, e.g. `src/app.rs`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct RepoPath(String);

impl RepoPath {
    /// Validate and construct a path.
    pub fn new(s: impl Into<String>) -> Result<Self, PathError> {
        let s = s.into();
        if s.is_empty() {
            return Err(PathError::Empty);
        }
        if s.starts_with('/') || s.ends_with('/') {
            return Err(PathError::LeadingOrTrailingSlash);
        }
        for c in s.split('/') {
            check_component(c)?;
        }
        Ok(Self(s))
    }

    /// The path as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Iterate over the path components.
    pub fn components(&self) -> impl Iterator<Item = PathComponent> + '_ {
        self.0.split('/').map(|c| PathComponent(c.to_owned()))
    }

    /// The final component (file name).
    #[must_use]
    pub fn file_name(&self) -> PathComponent {
        PathComponent(self.0.rsplit('/').next().unwrap_or(&self.0).to_owned())
    }

    /// The parent path, or `None` at the root.
    #[must_use]
    pub fn parent(&self) -> Option<RepoPath> {
        self.0.rfind('/').map(|i| RepoPath(self.0[..i].to_owned()))
    }

    /// Append a component.
    #[must_use]
    pub fn join(&self, c: &PathComponent) -> RepoPath {
        RepoPath(format!("{}/{}", self.0, c.0))
    }
}

impl fmt::Debug for RepoPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RepoPath({:?})", self.0)
    }
}

impl fmt::Display for RepoPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for RepoPath {
    type Error = PathError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&str> for RepoPath {
    type Error = PathError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<RepoPath> for String {
    fn from(p: RepoPath) -> Self {
        p.0
    }
}

/// A single path component (a file or directory name), e.g. `app.rs`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct PathComponent(String);

impl PathComponent {
    /// Validate and construct a component.
    pub fn new(s: impl Into<String>) -> Result<Self, PathError> {
        let s = s.into();
        if s.contains('/') {
            return Err(PathError::SlashInComponent);
        }
        check_component(&s)?;
        Ok(Self(s))
    }

    /// The component as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PathComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PathComponent({:?})", self.0)
    }
}

impl fmt::Display for PathComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for PathComponent {
    type Error = PathError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<PathComponent> for String {
    fn from(c: PathComponent) -> Self {
        c.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_paths() {
        RepoPath::new("src/app.rs").unwrap();
        RepoPath::new("README").unwrap();
        RepoPath::new("a/b/c/d").unwrap();
    }

    #[test]
    fn rejects_bad_paths() {
        assert_eq!(RepoPath::new(""), Err(PathError::Empty));
        assert_eq!(RepoPath::new("/a"), Err(PathError::LeadingOrTrailingSlash));
        assert_eq!(RepoPath::new("a/"), Err(PathError::LeadingOrTrailingSlash));
        assert_eq!(RepoPath::new("a//b"), Err(PathError::EmptyComponent));
        assert_eq!(
            RepoPath::new("a/../b"),
            Err(PathError::ReservedComponent("..".into()))
        );
        assert_eq!(
            RepoPath::new("./a"),
            Err(PathError::ReservedComponent(".".into()))
        );
        assert_eq!(RepoPath::new("a\0b"), Err(PathError::Nul));
        assert_eq!(PathComponent::new("a/b"), Err(PathError::SlashInComponent));
    }

    #[test]
    fn ordering_is_bytewise() {
        // "a-b" < "a/b" < "a0" in byte order, regardless of how a filesystem sorts.
        let mut v = [
            RepoPath::new("a0").unwrap(),
            RepoPath::new("a/b").unwrap(),
            RepoPath::new("a-b").unwrap(),
        ];
        v.sort();
        let s: Vec<_> = v.iter().map(RepoPath::as_str).collect();
        assert_eq!(s, ["a-b", "a/b", "a0"]);
    }

    #[test]
    fn parent_and_file_name() {
        let p = RepoPath::new("src/lib/app.rs").unwrap();
        assert_eq!(p.parent().unwrap().as_str(), "src/lib");
        assert_eq!(p.file_name().as_str(), "app.rs");
        assert!(RepoPath::new("README").unwrap().parent().is_none());
    }
}
