use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Maximum allowed path length (4096 bytes - reasonable for most filesystems)
const MAX_PATH_LENGTH: usize = 4096;

/// Validated repository path that has been sanitized and checked.
/// This type ensures all paths used in IPC communication are safe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValidatedPath(PathBuf);

impl ValidatedPath {
    /// Validates and converts a PathBuf into a ValidatedPath.
    pub fn new(path: PathBuf) -> Result<Self, PathValidationError> {
        // Check for empty path
        if path.as_os_str().is_empty() {
            return Err(PathValidationError::Empty);
        }

        // Check for overly long paths
        if path.as_os_str().len() > MAX_PATH_LENGTH {
            return Err(PathValidationError::TooLong {
                len: path.as_os_str().len(),
                max: MAX_PATH_LENGTH,
            });
        }

        // Normalize the path (resolve . and .. as much as possible without following symlinks)
        let normalized = normalize_path(&path)?;

        // Check for null bytes (invalid in paths)
        if normalized.as_os_str().to_string_lossy().contains('\0') {
            return Err(PathValidationError::ContainsNull);
        }

        // Check for suspicious escape sequences
        if contains_suspicious_patterns(&normalized) {
            return Err(PathValidationError::SuspiciousPattern);
        }

        Ok(Self(normalized))
    }

    /// Returns a reference to the underlying path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Consumes the ValidatedPath and returns the inner PathBuf.
    #[inline]
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl AsRef<Path> for ValidatedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Deref for ValidatedPath {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

/// Normalizes a path without following symlinks
fn normalize_path(path: &Path) -> Result<PathBuf, PathValidationError> {
    let mut result = PathBuf::new();
    let has_root = path.has_root() || path.is_absolute();

    for component in path.components() {
        match component {
            // Skip current directory references
            std::path::Component::CurDir => {}
            // Handle parent directory references
            std::path::Component::ParentDir => {
                // Can't go above the root - check if we're at root
                if result.as_os_str().is_empty() || (has_root && result.as_os_str() == "/") {
                    return Err(PathValidationError::PathTraversal);
                }
                result.pop();
            }
            // Root directory - just mark that we have a root
            std::path::Component::RootDir => {
                result.push("/");
            }
            // Normalize/normal components
            component @ std::path::Component::Normal(_) => {
                result.push(component);
            }
            // Prefix components (like drive letters on Windows) - pass through
            component => result.push(component.as_os_str()),
        }
    }

    Ok(result)
}

/// Checks for suspicious patterns in paths
fn contains_suspicious_patterns(path: &Path) -> bool {
    let s = path.as_os_str().to_string_lossy();

    // Check for escape sequences that could be used for injection
    if s.contains("${") || s.contains('`') {
        return true;
    }

    // Check for control characters
    for c in s.chars() {
        if c.is_control() {
            return true;
        }
    }

    false
}

/// Errors that can occur during path validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathValidationError {
    #[error("path is empty")]
    Empty,
    #[error("path too long: {len} bytes exceeds maximum of {max} bytes")]
    TooLong { len: usize, max: usize },
    #[error("path contains null byte")]
    ContainsNull,
    #[error("path contains suspicious pattern (escape sequences or control characters)")]
    SuspiciousPattern,
    #[error("path traversal outside repository root")]
    PathTraversal,
}

impl std::fmt::Display for ValidatedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl From<ValidatedPath> for PathBuf {
    fn from(val: ValidatedPath) -> Self {
        val.0
    }
}

// Make ValidatedPath serializable/deserializable by delegating to PathBuf
impl Serialize for ValidatedPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ValidatedPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let path = PathBuf::deserialize(deserializer)?;
        ValidatedPath::new(path).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normal_path_accepted() {
        let path = PathBuf::from("/home/user/repo");
        assert!(ValidatedPath::new(path).is_ok());
    }

    #[test]
    fn empty_path_rejected() {
        let path = PathBuf::new();
        assert!(matches!(
            ValidatedPath::new(path),
            Err(PathValidationError::Empty)
        ));
    }

    #[test]
    fn path_with_null_byte_rejected() {
        let mut path = PathBuf::from("/home/user/repo");
        path.push("\0");
        assert!(matches!(
            ValidatedPath::new(path),
            Err(PathValidationError::ContainsNull)
        ));
    }

    #[test]
    fn path_traversal_rejected() {
        let path = PathBuf::from("/home/user/../../../etc");
        assert!(matches!(
            ValidatedPath::new(path),
            Err(PathValidationError::PathTraversal)
        ));
    }

    #[test]
    fn control_characters_rejected() {
        let mut path = PathBuf::from("/home/user/repo");
        path.push("\x01");
        assert!(matches!(
            ValidatedPath::new(path),
            Err(PathValidationError::SuspiciousPattern)
        ));
    }

    #[test]
    fn path_with_dot_normalized() {
        let path = PathBuf::from("/home/user/./repo");
        let validated = ValidatedPath::new(path).unwrap();
        assert_eq!(validated.as_path(), Path::new("/home/user/repo"));
    }

    #[test]
    fn path_with_parent_normalized() {
        let path = PathBuf::from("/home/user/../user/repo");
        let validated = ValidatedPath::new(path).unwrap();
        assert_eq!(validated.as_path(), Path::new("/home/user/repo"));
    }

    #[test]
    fn escape_sequence_rejected() {
        let path = PathBuf::from("/home/user/${VAR}");
        assert!(matches!(
            ValidatedPath::new(path),
            Err(PathValidationError::SuspiciousPattern)
        ));
    }

    #[test]
    fn backtick_rejected() {
        let path = PathBuf::from("/home/user/`command`");
        assert!(matches!(
            ValidatedPath::new(path),
            Err(PathValidationError::SuspiciousPattern)
        ));
    }

    #[test]
    fn display_trait_works() {
        let path = PathBuf::from("/home/user/repo");
        let validated = ValidatedPath::new(path).unwrap();
        assert_eq!(format!("{}", validated), "/home/user/repo");
    }

    #[test]
    fn serde_roundtrip() {
        let path = PathBuf::from("/home/user/repo");
        let validated = ValidatedPath::new(path).unwrap();
        let bytes = bincode::serialize(&validated).unwrap();
        let decoded: ValidatedPath = bincode::deserialize(&bytes).unwrap();
        assert_eq!(validated.as_path(), decoded.as_path());
    }
}
