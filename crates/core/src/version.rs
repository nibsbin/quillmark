//! # Version Management
//!
//! Semantic versioning (MAJOR.MINOR.PATCH) for Quill template references.
//! Two-segment (`MAJOR.MINOR`) versions are also accepted; patch defaults to 0.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Semantic version number (MAJOR.MINOR.PATCH).
/// Two-segment form (`MAJOR.MINOR`) is also accepted; patch defaults to 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();

        if !matches!(parts.len(), 2 | 3) {
            return Err(format!(
                "Invalid version format '{}': expected MAJOR.MINOR.PATCH or MAJOR.MINOR (e.g., '2.1.0' or '2.1')",
                s
            ));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid major version '{}': must be a number", parts[0]))?;

        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid minor version '{}': must be a number", parts[1]))?;

        let patch = if parts.len() == 3 {
            parts[2]
                .parse::<u32>()
                .map_err(|_| format!("Invalid patch version '{}': must be a number", parts[2]))?
        } else {
            0
        };

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
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
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                other => other,
            },
            other => other,
        }
    }
}

/// Specifies which version of a Quill template to use.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VersionSelector {
    /// Match exactly this version (e.g., "@2.1.0")
    Exact(Version),
    /// Match latest patch version in this minor series (e.g., "@2.1")
    Minor(u32, u32),
    /// Match latest minor/patch version in this major series (e.g., "@2")
    Major(u32),
    /// Match the highest version available (e.g., "@latest" or unspecified)
    Latest,
}

impl VersionSelector {
    /// Whether `v` satisfies this selector: `Exact` the identical version,
    /// `Minor` any patch in the `MAJOR.MINOR` series, `Major` any version in the
    /// `MAJOR` series, `Latest` anything. A compatibility check, not resolution
    /// — a `false` is the `quill::version_mismatch` render error.
    pub fn matches(&self, v: Version) -> bool {
        match self {
            VersionSelector::Exact(want) => *want == v,
            VersionSelector::Minor(major, minor) => v.major == *major && v.minor == *minor,
            VersionSelector::Major(major) => v.major == *major,
            VersionSelector::Latest => true,
        }
    }
}

impl FromStr for VersionSelector {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version_str = s.strip_prefix('@').unwrap_or(s);

        if version_str.is_empty() || version_str == "latest" {
            return Ok(VersionSelector::Latest);
        }

        let parts: Vec<&str> = version_str.split('.').collect();

        match parts.len() {
            3 => {
                let version = Version::from_str(version_str)?;
                Ok(VersionSelector::Exact(version))
            }
            2 => {
                let major = parts[0].parse::<u32>().map_err(|_| {
                    format!("Invalid major version '{}': must be a number", parts[0])
                })?;
                let minor = parts[1].parse::<u32>().map_err(|_| {
                    format!("Invalid minor version '{}': must be a number", parts[1])
                })?;
                Ok(VersionSelector::Minor(major, minor))
            }
            1 => {
                let major = version_str.parse::<u32>().map_err(|_| {
                    format!(
                        "Invalid version selector '{}': expected number, MAJOR.MINOR, MAJOR.MINOR.PATCH, or 'latest'",
                        version_str
                    )
                })?;
                Ok(VersionSelector::Major(major))
            }
            _ => Err(format!(
                "Invalid version selector '{}': expected number, MAJOR.MINOR, MAJOR.MINOR.PATCH, or 'latest'",
                version_str
            )),
        }
    }
}

impl fmt::Display for VersionSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionSelector::Exact(v) => write!(f, "@{}", v),
            VersionSelector::Minor(major, minor) => write!(f, "@{}.{}", major, minor),
            VersionSelector::Major(m) => write!(f, "@{}", m),
            VersionSelector::Latest => write!(f, "@latest"),
        }
    }
}

/// Canonical, author-facing `$quill` reference grammar.
const QUILL_REF_HINT: &str = "A $quill reference is `<name>` or `<name>@<selector>`. \
The name must match `[a-z_][a-z0-9_]*` (start with a lowercase letter or underscore, then \
lowercase letters, digits, or underscores). The optional version selector is \
`@MAJOR.MINOR.PATCH` (exact), `@MAJOR.MINOR` (latest patch in that minor series), `@MAJOR` \
(latest in that major series), or `@latest`; omitting the selector means latest.";

/// Single source of truth for the grammar [`QuillReference::from_str`] enforces:
/// bindings surface it (schema `describe`, validation hints) and it rides as the
/// `hint` on the `parse::invalid_quill_reference` diagnostic, so error and
/// describe text can't drift from the parser. Sibling of `document`'s
/// `FORMAT_RULES` / `blueprint_instruction`.
pub fn quill_ref_hint() -> &'static str {
    QUILL_REF_HINT
}

/// Complete reference to a Quill template with name and version selector.
///
/// Name charset: `[a-z_][a-z0-9_]*`. Selector defaults to `Latest` when omitted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct QuillReference {
    pub name: String,
    pub selector: VersionSelector,
}

/// Which half of a [`QuillReference`] a quill's identity failed.
///
/// The two arms are the two `quill::*_mismatch` codes, minus the diagnostic:
/// [`QuillReference::check`] returns this so an enforcement site can word a
/// coded error while a consumer-side resolver reads a plain `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RefMismatch {
    /// The quill's name differs from the reference's — `quill::name_mismatch`.
    Name,
    /// Names agree; the version falls outside the selector —
    /// `quill::version_mismatch`.
    Version,
}

impl QuillReference {
    pub fn new(name: String, selector: VersionSelector) -> Self {
        Self { name, selector }
    }

    /// Whether a quill named `name` at `version` satisfies this reference.
    ///
    /// The predicate the engine enforces with, exposed for the layer that does
    /// *resolution* — picking a quill out of a set it already holds — which
    /// this crate deliberately does not do (see `VERSIONING.md` §"Document
    /// Syntax"). A `true` here is the guarantee that `Quillmark::open` will not
    /// raise a `quill::*_mismatch` for this pairing.
    pub fn satisfied_by(&self, name: &str, version: &str) -> bool {
        self.check(name, version).is_ok()
    }

    /// [`satisfied_by`](Self::satisfied_by) with the failing half named.
    ///
    /// Name is the prerequisite — a selector belongs to a *named* quill — so a
    /// name mismatch short-circuits and the version is left unevaluated.
    /// `version` is the quill's declared version string, parsed leniently: one
    /// that doesn't parse skips the selector check rather than failing it,
    /// since load already validated it and a second opinion here would reject
    /// a pairing the engine accepts.
    pub fn check(&self, name: &str, version: &str) -> Result<(), RefMismatch> {
        if self.name != name {
            return Err(RefMismatch::Name);
        }

        let Ok(version) = Version::from_str(version) else {
            return Ok(());
        };
        if !self.selector.matches(version) {
            return Err(RefMismatch::Version);
        }

        Ok(())
    }

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
        let separator_idx = s.find('@');

        let (name_part, version_part_opt) = match separator_idx {
            Some(idx) => (&s[..idx], Some(&s[idx + 1..])),
            None => (s, None),
        };

        if name_part.is_empty() {
            return Err("Quill name cannot be empty".to_string());
        }

        let name = name_part.to_string();

        // Same charset as a card kind, leading underscore included — one
        // predicate, in `document::meta`. (Quill *config* names are stricter:
        // `config.rs` rejects a leading underscore, so its predicate is not
        // interchangeable with this one.)
        if !crate::document::is_valid_kind_name(&name) {
            return Err(format!(
                "Invalid Quill name '{}': must match [a-z_][a-z0-9_]*",
                name
            ));
        }

        let selector = if let Some(version_part) = version_part_opt {
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
        let v = Version::from_str("2.1.0").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
        assert_eq!(v.to_string(), "2.1.0");

        let v2 = Version::from_str("1.2.3").unwrap();
        assert_eq!(v2.major, 1);
        assert_eq!(v2.minor, 2);
        assert_eq!(v2.patch, 3);
        assert_eq!(v2.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_parsing_two_segment_backward_compat() {
        let v = Version::from_str("2.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
        assert_eq!(v.to_string(), "2.1.0");
    }

    #[test]
    fn test_version_invalid() {
        assert!(Version::from_str("2").is_err());
        assert!(Version::from_str("2.1.0.0").is_err());
        assert!(Version::from_str("abc").is_err());
        assert!(Version::from_str("2.x").is_err());
        assert!(Version::from_str("2.1.x").is_err());
    }

    #[test]
    fn test_version_ordering() {
        let v1_0_0 = Version::new(1, 0, 0);
        let v1_0_1 = Version::new(1, 0, 1);
        let v1_1_0 = Version::new(1, 1, 0);
        let v2_0_0 = Version::new(2, 0, 0);
        let v2_1_0 = Version::new(2, 1, 0);

        assert!(v1_0_0 < v1_0_1);
        assert!(v1_0_1 < v1_1_0);
        assert!(v1_1_0 < v2_0_0);
        assert!(v2_0_0 < v2_1_0);
        assert_eq!(v1_0_0, v1_0_0);
    }

    #[test]
    fn test_version_selector_parsing() {
        let exact = VersionSelector::from_str("@2.1.0").unwrap();
        assert_eq!(exact, VersionSelector::Exact(Version::new(2, 1, 0)));

        let minor = VersionSelector::from_str("@2.1").unwrap();
        assert_eq!(minor, VersionSelector::Minor(2, 1));

        let major = VersionSelector::from_str("@2").unwrap();
        assert_eq!(major, VersionSelector::Major(2));

        let latest1 = VersionSelector::from_str("@latest").unwrap();
        assert_eq!(latest1, VersionSelector::Latest);

        // Empty string also means Latest
        let latest2 = VersionSelector::from_str("").unwrap();
        assert_eq!(latest2, VersionSelector::Latest);
    }

    #[test]
    fn test_version_selector_without_at() {
        let exact = VersionSelector::from_str("2.1.0").unwrap();
        assert_eq!(exact, VersionSelector::Exact(Version::new(2, 1, 0)));

        let minor = VersionSelector::from_str("2.1").unwrap();
        assert_eq!(minor, VersionSelector::Minor(2, 1));

        let major = VersionSelector::from_str("2").unwrap();
        assert_eq!(major, VersionSelector::Major(2));
    }

    #[test]
    fn test_version_selector_matches() {
        let v2_1_0 = Version::new(2, 1, 0);
        let v2_1_3 = Version::new(2, 1, 3);
        let v2_2_0 = Version::new(2, 2, 0);
        let v3_0_0 = Version::new(3, 0, 0);

        // Exact: only the identical version satisfies.
        let exact = VersionSelector::Exact(v2_1_0);
        assert!(exact.matches(v2_1_0));
        assert!(!exact.matches(v2_1_3));
        assert!(!exact.matches(v2_2_0));

        // Minor: any patch within the 2.1 series.
        let minor = VersionSelector::Minor(2, 1);
        assert!(minor.matches(v2_1_0));
        assert!(minor.matches(v2_1_3));
        assert!(!minor.matches(v2_2_0));
        assert!(!minor.matches(v3_0_0));

        // Major: any minor/patch within the 2 series.
        let major = VersionSelector::Major(2);
        assert!(major.matches(v2_1_0));
        assert!(major.matches(v2_2_0));
        assert!(!major.matches(v3_0_0));

        // Latest: matches anything.
        let latest = VersionSelector::Latest;
        assert!(latest.matches(v2_1_0));
        assert!(latest.matches(v3_0_0));
    }

    #[test]
    fn test_version_selector_display() {
        assert_eq!(
            VersionSelector::Exact(Version::new(2, 1, 0)).to_string(),
            "@2.1.0"
        );
        assert_eq!(VersionSelector::Minor(2, 1).to_string(), "@2.1");
        assert_eq!(VersionSelector::Major(2).to_string(), "@2");
        assert_eq!(VersionSelector::Latest.to_string(), "@latest");
    }

    #[test]
    fn test_quill_reference_parsing() {
        let ref1 = QuillReference::from_str("resume_template@2.1.0").unwrap();
        assert_eq!(ref1.name, "resume_template");
        assert_eq!(ref1.selector, VersionSelector::Exact(Version::new(2, 1, 0)));

        let ref1b = QuillReference::from_str("resume_template@2.1").unwrap();
        assert_eq!(ref1b.selector, VersionSelector::Minor(2, 1));

        let ref2 = QuillReference::from_str("resume_template@2").unwrap();
        assert_eq!(ref2.selector, VersionSelector::Major(2));

        let ref3 = QuillReference::from_str("resume_template@latest").unwrap();
        assert_eq!(ref3.selector, VersionSelector::Latest);

        // No @ suffix — defaults to Latest
        let ref4 = QuillReference::from_str("resume_template").unwrap();
        assert_eq!(ref4.name, "resume_template");
        assert_eq!(ref4.selector, VersionSelector::Latest);
    }

    #[test]
    fn test_quill_reference_invalid_names() {
        assert!(QuillReference::from_str("Resume@2.1.0").is_err());
        assert!(QuillReference::from_str("1resume@2.1.0").is_err());
        assert!(QuillReference::from_str("resume-template@2.1.0").is_err());
        assert!(QuillReference::from_str("resume.template@2.1.0").is_err());
        assert!(QuillReference::from_str("resume_template@2.1.0").is_ok());
        assert!(QuillReference::from_str("_private@2.1.0").is_ok());
        assert!(QuillReference::from_str("template2@2.1.0").is_ok());
    }

    #[test]
    fn test_quill_reference_display() {
        let ref1 = QuillReference::new(
            "resume".to_string(),
            VersionSelector::Exact(Version::new(2, 1, 0)),
        );
        assert_eq!(ref1.to_string(), "resume@2.1.0");

        let ref1b = QuillReference::new("resume".to_string(), VersionSelector::Minor(2, 1));
        assert_eq!(ref1b.to_string(), "resume@2.1");

        let ref2 = QuillReference::new("resume".to_string(), VersionSelector::Major(2));
        assert_eq!(ref2.to_string(), "resume@2");

        let ref3 = QuillReference::new("resume".to_string(), VersionSelector::Latest);
        assert_eq!(ref3.to_string(), "resume");
    }

    #[test]
    fn test_quill_ref_hint_describes_the_grammar() {
        let hint = quill_ref_hint();
        assert!(!hint.is_empty());
        // Pin the charset and selector forms so the hint can't drift from `from_str`.
        assert!(hint.contains("[a-z_][a-z0-9_]*"), "got: {hint}");
        assert!(hint.contains("@latest"), "got: {hint}");
        assert!(hint.contains("@MAJOR.MINOR.PATCH"), "got: {hint}");
    }

    /// One case per selector arm, satisfied and not — the table a consumer
    /// would otherwise re-derive in JS or Python.
    #[test]
    fn satisfied_by_covers_every_selector_arm() {
        let cases = [
            ("resume", "2.1.0", true),
            ("resume@latest", "2.1.0", true),
            ("resume@2", "2.9.9", true),
            ("resume@2", "3.0.0", false),
            ("resume@2.1", "2.1.7", true),
            ("resume@2.1", "2.2.0", false),
            ("resume@2.1.0", "2.1.0", true),
            ("resume@2.1.0", "2.1.1", false),
        ];

        for (reference, version, want) in cases {
            let parsed = QuillReference::from_str(reference).unwrap();
            assert_eq!(
                parsed.satisfied_by("resume", version),
                want,
                "{reference} against {version}"
            );
        }
    }

    #[test]
    fn check_names_the_failing_half() {
        let reference = QuillReference::from_str("resume@2.1.0").unwrap();

        assert_eq!(reference.check("resume", "2.1.0"), Ok(()));
        assert_eq!(reference.check("letter", "2.1.0"), Err(RefMismatch::Name));
        assert_eq!(reference.check("resume", "3.0.0"), Err(RefMismatch::Version));
    }

    /// Name is the prerequisite: a pairing that fails both halves reports the
    /// name, since a selector belongs to a *named* quill.
    #[test]
    fn check_short_circuits_on_the_name() {
        let reference = QuillReference::from_str("resume@2.1.0").unwrap();
        assert_eq!(reference.check("letter", "9.9.9"), Err(RefMismatch::Name));
    }

    /// A version the engine could not parse skips the selector rather than
    /// failing it — the check agrees with `check_quill_reference`, which
    /// accepts that pairing rather than rejecting one load already blessed.
    #[test]
    fn check_skips_an_unparseable_version() {
        let reference = QuillReference::from_str("resume@2.1.0").unwrap();
        assert!(reference.satisfied_by("resume", "not-a-version"));
    }

    /// Two-segment declarations parse, so they compare by the patch they mean.
    #[test]
    fn satisfied_by_accepts_a_two_segment_version() {
        let reference = QuillReference::from_str("resume@2.1.0").unwrap();
        assert!(reference.satisfied_by("resume", "2.1"));
    }
}
