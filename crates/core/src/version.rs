//! # Version Management
//!
//! Version types and parsing for Quill template versioning.
//!
//! ## Overview
//!
//! This module provides types for managing Quill template versions using a
//! two-segment versioning scheme (MAJOR.MINOR). This is simpler than semantic
//! versioning while still providing meaningful compatibility signaling.
//!
//! ## Key Types
//!
//! - [`Version`]: Two-segment version number (MAJOR.MINOR)
//! - [`VersionSelector`]: Specifies which version to use (exact, major, or latest)
//! - [`QuillReference`]: Complete reference to a Quill with name and version
//!
//! ## Examples
//!
//! ### Parsing Versions
//!
//! ```
//! use quillmark_core::version::Version;
//! use std::str::FromStr;
//!
//! let v = Version::from_str("2.1").unwrap();
//! assert_eq!(v.major, 2);
//! assert_eq!(v.minor, 1);
//! assert_eq!(v.to_string(), "2.1");
//! ```
//!
//! ### Version Comparison
//!
//! ```
//! use quillmark_core::version::Version;
//! use std::str::FromStr;
//!
//! let v1 = Version::from_str("1.0").unwrap();
//! let v2 = Version::from_str("2.1").unwrap();
//! assert!(v1 < v2);
//! ```
//!
//! ### Parsing Quill References
//!
//! ```
//! use quillmark_core::version::QuillReference;
//! use std::str::FromStr;
//!
//! let ref1 = QuillReference::from_str("resume_template@2.1").unwrap();
//! assert_eq!(ref1.name, "resume_template");
//!
//! let ref2 = QuillReference::from_str("resume_template@2").unwrap();
//! let ref3 = QuillReference::from_str("resume_template@latest").unwrap();
//! let ref4 = QuillReference::from_str("resume_template").unwrap();
//! ```

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Two-segment version number (MAJOR.MINOR)
///
/// Versions use a simple two-segment scheme where:
/// - MAJOR indicates breaking changes
/// - MINOR indicates compatible changes (features, fixes, improvements)
///
/// # Examples
///
/// ```
/// use quillmark_core::version::Version;
/// use std::str::FromStr;
///
/// let v = Version::new(2, 1);
/// assert_eq!(v.to_string(), "2.1");
///
/// let parsed = Version::from_str("2.1").unwrap();
/// assert_eq!(parsed, v);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    /// Major version number (breaking changes)
    pub major: u32,
    /// Minor version number (compatible changes)
    pub minor: u32,
}

impl Version {
    /// Create a new version
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid version format '{}': expected MAJOR.MINOR (e.g., '2.1')",
                s
            ));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid major version '{}': must be a number", parts[0]))?;

        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid minor version '{}': must be a number", parts[1]))?;

        Ok(Version { major, minor })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            other => other,
        }
    }
}

/// Specifies which version of a Quill template to use
///
/// # Examples
///
/// ```
/// use quillmark_core::version::VersionSelector;
/// use std::str::FromStr;
///
/// let exact = VersionSelector::from_str("@2.1").unwrap();
/// let major = VersionSelector::from_str("@2").unwrap();
/// let latest = VersionSelector::from_str("@latest").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionSelector {
    /// Match exactly this version (e.g., "@2.1")
    Exact(Version),
    /// Match latest minor version in this major series (e.g., "@2")
    Major(u32),
    /// Match the highest version available (e.g., "@latest" or unspecified)
    Latest,
}

impl FromStr for VersionSelector {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strip leading @ if present
        let version_str = s.strip_prefix('@').unwrap_or(s);

        if version_str.is_empty() || version_str == "latest" {
            return Ok(VersionSelector::Latest);
        }

        // Try parsing as full version (MAJOR.MINOR)
        if version_str.contains('.') {
            let version = Version::from_str(version_str)?;
            return Ok(VersionSelector::Exact(version));
        }

        // Parse as major-only version
        let major = version_str.parse::<u32>().map_err(|_| {
            format!(
                "Invalid version selector '{}': expected number, MAJOR.MINOR, or 'latest'",
                version_str
            )
        })?;

        Ok(VersionSelector::Major(major))
    }
}

impl fmt::Display for VersionSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionSelector::Exact(v) => write!(f, "@{}", v),
            VersionSelector::Major(m) => write!(f, "@{}", m),
            VersionSelector::Latest => write!(f, "@latest"),
        }
    }
}

/// Complete reference to a Quill template with name and version selector
///
/// # Examples
///
/// ```
/// use quillmark_core::version::QuillReference;
/// use std::str::FromStr;
///
/// let ref1 = QuillReference::from_str("resume_template@2.1").unwrap();
/// assert_eq!(ref1.name, "resume_template");
///
/// let ref2 = QuillReference::from_str("resume_template").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuillReference {
    /// Template name (e.g., "resume_template")
    pub name: String,
    /// Version selector (defaults to Latest if not specified)
    pub selector: VersionSelector,
}

impl QuillReference {
    /// Create a new QuillReference
    pub fn new(name: String, selector: VersionSelector) -> Self {
        Self { name, selector }
    }

    /// Create a QuillReference with Latest selector
    pub fn latest(name: String) -> Self {
        Self {
            name,
            selector: VersionSelector::Latest,
        }
    }
}

impl FromStr for QuillReference {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split on @ to separate name from version
        let parts: Vec<&str> = s.split('@').collect();

        if parts.is_empty() {
            return Err("Empty Quill reference".to_string());
        }

        let name = parts[0].to_string();
        if name.is_empty() {
            return Err("Quill name cannot be empty".to_string());
        }

        // Validate name format: [a-z_][a-z0-9_]*
        if !name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        {
            return Err(format!(
                "Invalid Quill name '{}': must start with lowercase letter or underscore",
                name
            ));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(format!(
                "Invalid Quill name '{}': must contain only lowercase letters, digits, and underscores",
                name
            ));
        }

        // Parse version selector if present
        let selector = if parts.len() > 1 {
            // Reconstruct version part (in case there were multiple @ symbols)
            let version_part = parts[1..].join("@");
            VersionSelector::from_str(&format!("@{}", version_part))?
        } else {
            VersionSelector::Latest
        };

        Ok(QuillReference { name, selector })
    }
}

impl fmt::Display for QuillReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.selector {
            VersionSelector::Latest => write!(f, "{}", self.name),
            _ => write!(f, "{}{}", self.name, self.selector),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = Version::from_str("2.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.to_string(), "2.1");
    }

    #[test]
    fn test_version_invalid() {
        assert!(Version::from_str("2").is_err());
        assert!(Version::from_str("2.1.0").is_err());
        assert!(Version::from_str("abc").is_err());
        assert!(Version::from_str("2.x").is_err());
    }

    #[test]
    fn test_version_ordering() {
        let v1_0 = Version::new(1, 0);
        let v1_1 = Version::new(1, 1);
        let v2_0 = Version::new(2, 0);
        let v2_1 = Version::new(2, 1);

        assert!(v1_0 < v1_1);
        assert!(v1_1 < v2_0);
        assert!(v2_0 < v2_1);
        assert_eq!(v1_0, v1_0);
    }

    #[test]
    fn test_version_selector_parsing() {
        let exact = VersionSelector::from_str("@2.1").unwrap();
        assert_eq!(exact, VersionSelector::Exact(Version::new(2, 1)));

        let major = VersionSelector::from_str("@2").unwrap();
        assert_eq!(major, VersionSelector::Major(2));

        let latest1 = VersionSelector::from_str("@latest").unwrap();
        assert_eq!(latest1, VersionSelector::Latest);

        let latest2 = VersionSelector::from_str("").unwrap();
        assert_eq!(latest2, VersionSelector::Latest);
    }

    #[test]
    fn test_version_selector_without_at() {
        let exact = VersionSelector::from_str("2.1").unwrap();
        assert_eq!(exact, VersionSelector::Exact(Version::new(2, 1)));

        let major = VersionSelector::from_str("2").unwrap();
        assert_eq!(major, VersionSelector::Major(2));
    }

    #[test]
    fn test_quill_reference_parsing() {
        let ref1 = QuillReference::from_str("resume_template@2.1").unwrap();
        assert_eq!(ref1.name, "resume_template");
        assert_eq!(ref1.selector, VersionSelector::Exact(Version::new(2, 1)));

        let ref2 = QuillReference::from_str("resume_template@2").unwrap();
        assert_eq!(ref2.name, "resume_template");
        assert_eq!(ref2.selector, VersionSelector::Major(2));

        let ref3 = QuillReference::from_str("resume_template@latest").unwrap();
        assert_eq!(ref3.name, "resume_template");
        assert_eq!(ref3.selector, VersionSelector::Latest);

        let ref4 = QuillReference::from_str("resume_template").unwrap();
        assert_eq!(ref4.name, "resume_template");
        assert_eq!(ref4.selector, VersionSelector::Latest);
    }

    #[test]
    fn test_quill_reference_invalid_names() {
        // Must start with lowercase or underscore
        assert!(QuillReference::from_str("Resume@2.1").is_err());
        assert!(QuillReference::from_str("1resume@2.1").is_err());

        // Must contain only lowercase, digits, underscores
        assert!(QuillReference::from_str("resume-template@2.1").is_err());
        assert!(QuillReference::from_str("resume.template@2.1").is_err());

        // Valid names
        assert!(QuillReference::from_str("resume_template@2.1").is_ok());
        assert!(QuillReference::from_str("_private@2.1").is_ok());
        assert!(QuillReference::from_str("template2@2.1").is_ok());
    }

    #[test]
    fn test_quill_reference_display() {
        let ref1 = QuillReference::new(
            "resume".to_string(),
            VersionSelector::Exact(Version::new(2, 1)),
        );
        assert_eq!(ref1.to_string(), "resume@2.1");

        let ref2 = QuillReference::new("resume".to_string(), VersionSelector::Major(2));
        assert_eq!(ref2.to_string(), "resume@2");

        let ref3 = QuillReference::new("resume".to_string(), VersionSelector::Latest);
        assert_eq!(ref3.to_string(), "resume");
    }
}
