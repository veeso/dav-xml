// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed values for the RFC 4918 section 10 headers.

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn overwrite_round_trips() {
        assert_eq!(Overwrite::True.to_string(), "T");
        assert_eq!("F".parse::<Overwrite>().unwrap(), Overwrite::False);
        "x".parse::<Overwrite>().unwrap_err();
    }

    #[test]
    fn untagged_if_list() {
        let header = If::untagged("urn:uuid:181d4fae-7d8c-11d0-a765-00a0c91e6bf2");
        assert_eq!(
            header.to_string(),
            "(<urn:uuid:181d4fae-7d8c-11d0-a765-00a0c91e6bf2>)"
        );
    }

    #[test]
    fn tagged_if_list_with_not_and_etag() {
        let header = If::new().with_list(
            IfList::tagged("http://www.example.com/specs/rfc2518.doc".parse().unwrap())
                .token("urn:uuid:181d4fae-7d8c-11d0-a765-00a0c91e6bf2")
                .not_etag(ETag::strong("4217")),
        );
        assert_eq!(
            header.to_string(),
            "<http://www.example.com/specs/rfc2518.doc> (<urn:uuid:181d4fae-7d8c-11d0-a765-00a0c91e6bf2> Not [\"4217\"])"
        );
    }

    #[test]
    fn multiple_lists_are_space_separated() {
        let header = If::new()
            .with_list(IfList::untagged().token("a"))
            .with_list(IfList::untagged().token("b"));
        assert_eq!(header.to_string(), "(<a>) (<b>)");
    }

    #[test]
    fn lock_token_header_round_trips() {
        let token: LockTokenHeader = "<urn:x>".parse().unwrap();
        assert_eq!(token.0, "urn:x");
        assert_eq!(token.to_string(), "<urn:x>");
        assert_eq!("urn:y".parse::<LockTokenHeader>().unwrap().0, "urn:y");
    }

    #[test]
    fn lock_token_header_rejects_asymmetric_brackets() {
        "<urn:x".parse::<LockTokenHeader>().unwrap_err();
        "urn:x>".parse::<LockTokenHeader>().unwrap_err();
        "<".parse::<LockTokenHeader>().unwrap_err();
        ">".parse::<LockTokenHeader>().unwrap_err();
    }

    #[test]
    fn dav_header_lists_classes() {
        let dav: DavHeader = "1, 2, 3, extended-mkcol".parse().unwrap();
        assert_eq!(dav.classes, ["1", "2", "3", "extended-mkcol"]);
        assert!(dav.supports_locking());
        assert!(!"1".parse::<DavHeader>().unwrap().supports_locking());
    }

    #[test]
    fn timeout_header_round_trips() {
        let header = TimeoutHeader(vec![Timeout::Seconds(600), Timeout::Infinite]);
        assert_eq!(header.to_string(), "Second-600, Infinite");
        assert_eq!(
            "Second-5, Infinite".parse::<TimeoutHeader>().unwrap(),
            header_of(vec![Timeout::Seconds(5), Timeout::Infinite])
        );
    }

    fn header_of(timeouts: Vec<Timeout>) -> TimeoutHeader {
        TimeoutHeader(timeouts)
    }

    #[test]
    fn timeout_header_rejects_garbage_and_reports_the_original_value() {
        let error = "Second-abc".parse::<TimeoutHeader>().unwrap_err();
        assert_eq!(error.to_string(), "invalid Timeout header: Second-abc");
    }
}

use std::fmt::Display;
use std::str::FromStr;

use dav_xml::elements::Timeout;
use dav_xml::properties::ETag;
use http::HeaderName;

/// `DAV` (section 10.1).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::DAV;
///
/// assert_eq!(DAV.as_str(), "dav");
/// ```
pub const DAV: HeaderName = HeaderName::from_static("dav");
/// `Depth` (section 10.2).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::DEPTH;
///
/// assert_eq!(DEPTH.as_str(), "depth");
/// ```
pub const DEPTH: HeaderName = HeaderName::from_static("depth");
/// `Destination` (section 10.3).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::DESTINATION;
///
/// assert_eq!(DESTINATION.as_str(), "destination");
/// ```
pub const DESTINATION: HeaderName = HeaderName::from_static("destination");
/// `If` (section 10.4).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::IF;
///
/// assert_eq!(IF.as_str(), "if");
/// ```
pub const IF: HeaderName = HeaderName::from_static("if");
/// `Lock-Token` (section 10.5).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::LOCK_TOKEN;
///
/// assert_eq!(LOCK_TOKEN.as_str(), "lock-token");
/// ```
pub const LOCK_TOKEN: HeaderName = HeaderName::from_static("lock-token");
/// `Overwrite` (section 10.6).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::OVERWRITE;
///
/// assert_eq!(OVERWRITE.as_str(), "overwrite");
/// ```
pub const OVERWRITE: HeaderName = HeaderName::from_static("overwrite");
/// `Timeout` (section 10.7).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::TIMEOUT;
///
/// assert_eq!(TIMEOUT.as_str(), "timeout");
/// ```
pub const TIMEOUT: HeaderName = HeaderName::from_static("timeout");

/// A header value that does not follow the RFC grammar.
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::Overwrite;
///
/// let error = "maybe".parse::<Overwrite>().unwrap_err();
/// assert_eq!(error.to_string(), "invalid Overwrite header: maybe");
/// ```
#[derive(Debug, thiserror::Error)]
#[error("invalid {header} header: {value}")]
pub struct InvalidHeader {
    header: &'static str,
    value: String,
}

/// `Overwrite` header value ([RFC 4918 section 10.6](https://www.rfc-editor.org/rfc/rfc4918#section-10.6)).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::Overwrite;
///
/// assert_eq!(Overwrite::True.to_string(), "T");
/// assert_eq!("F".parse::<Overwrite>().unwrap(), Overwrite::False);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overwrite {
    /// `T`: replace an existing destination.
    #[default]
    True,
    /// `F`: fail with 412 if the destination exists.
    False,
}

impl Display for Overwrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::True => "T",
            Self::False => "F",
        })
    }
}

impl FromStr for Overwrite {
    type Err = InvalidHeader;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "T" | "t" => Ok(Self::True),
            "F" | "f" => Ok(Self::False),
            _ => Err(InvalidHeader {
                header: "Overwrite",
                value: s.to_owned(),
            }),
        }
    }
}

/// One condition inside an [`IfList`].
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::IfCondition;
///
/// let condition = IfCondition::Token {
///     token: "urn:x".to_owned(),
///     not: false,
/// };
/// assert_eq!(condition, IfCondition::Token { token: "urn:x".to_owned(), not: false });
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IfCondition {
    /// A state token, usually a lock token.
    Token {
        /// The token, without the enclosing `<` `>` coded-URL delimiters.
        token: String,
        /// Whether the condition is negated (`Not`).
        not: bool,
    },
    /// An entity tag.
    ETag {
        /// The entity tag.
        etag: ETag,
        /// Whether the condition is negated (`Not`).
        not: bool,
    },
}

/// One parenthesised list of an [`If`] header, optionally tagged with a
/// resource.
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::{If, IfList};
///
/// let header = If::new().with_list(IfList::untagged().token("urn:x"));
/// assert_eq!(header.to_string(), "(<urn:x>)");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IfList {
    resource: Option<http::Uri>,
    conditions: Vec<IfCondition>,
}

impl IfList {
    /// An untagged list, applying to whichever resource the request targets.
    #[must_use]
    pub fn untagged() -> Self {
        Self::default()
    }

    /// A list tagged to `resource`.
    #[must_use]
    pub fn tagged(resource: http::Uri) -> Self {
        Self {
            resource: Some(resource),
            conditions: Vec::new(),
        }
    }

    /// Require the presence of the given state token.
    #[must_use]
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.conditions.push(IfCondition::Token {
            token: token.into(),
            not: false,
        });
        self
    }

    /// Require the absence of the given state token.
    #[must_use]
    pub fn not_token(mut self, token: impl Into<String>) -> Self {
        self.conditions.push(IfCondition::Token {
            token: token.into(),
            not: true,
        });
        self
    }

    /// Require a match against the given entity tag.
    #[must_use]
    pub fn etag(mut self, etag: ETag) -> Self {
        self.conditions.push(IfCondition::ETag { etag, not: false });
        self
    }

    /// Require the given entity tag to not match.
    #[must_use]
    pub fn not_etag(mut self, etag: ETag) -> Self {
        self.conditions.push(IfCondition::ETag { etag, not: true });
        self
    }
}

/// The `If` header ([RFC 4918 section 10.4](https://www.rfc-editor.org/rfc/rfc4918#section-10.4)).
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::If;
///
/// let header = If::untagged("urn:uuid:181d4fae-7d8c-11d0-a765-00a0c91e6bf2");
/// assert_eq!(
///     header.to_string(),
///     "(<urn:uuid:181d4fae-7d8c-11d0-a765-00a0c91e6bf2>)"
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct If {
    lists: Vec<IfList>,
}

impl If {
    /// An empty header, carrying no lists.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `(<token>)`: a single untagged list requiring `token`.
    #[must_use]
    pub fn untagged(token: impl Into<String>) -> Self {
        Self::new().with_list(IfList::untagged().token(token))
    }

    /// `<resource> (<token>)`: a single list tagged to `resource`, requiring
    /// `token`.
    #[must_use]
    pub fn tagged(resource: http::Uri, token: impl Into<String>) -> Self {
        Self::new().with_list(IfList::tagged(resource).token(token))
    }

    /// Append `list` to the header.
    #[must_use]
    pub fn with_list(mut self, list: IfList) -> Self {
        self.lists.push(list);
        self
    }

    /// Whether the header carries no lists.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lists.is_empty()
    }
}

impl Display for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, list) in self.lists.iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            if let Some(resource) = &list.resource {
                write!(f, "<{resource}> ")?;
            }
            f.write_str("(")?;
            for (cindex, condition) in list.conditions.iter().enumerate() {
                if cindex > 0 {
                    f.write_str(" ")?;
                }
                match condition {
                    IfCondition::Token { token, not } => {
                        if *not {
                            f.write_str("Not ")?;
                        }
                        write!(f, "<{token}>")?;
                    }
                    IfCondition::ETag { etag, not } => {
                        if *not {
                            f.write_str("Not ")?;
                        }
                        write!(f, "[{etag}]")?;
                    }
                }
            }
            f.write_str(")")?;
        }
        Ok(())
    }
}

/// `Lock-Token` header value ([RFC 4918 section 10.5](https://www.rfc-editor.org/rfc/rfc4918#section-10.5)): a coded URL `<token>`.
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::LockTokenHeader;
///
/// let token: LockTokenHeader = "<urn:x>".parse().unwrap();
/// assert_eq!(token.0, "urn:x");
/// assert_eq!(token.to_string(), "<urn:x>");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockTokenHeader(
    /// The token, without the enclosing `<` `>` coded-URL delimiters.
    pub String,
);

impl Display for LockTokenHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{token}>", token = self.0)
    }
}

impl FromStr for LockTokenHeader {
    type Err = InvalidHeader;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let invalid = || InvalidHeader {
            header: "Lock-Token",
            value: s.to_owned(),
        };
        // Bracket-stripping is symmetric: either both `<` and `>` are
        // present and get stripped, or neither is and `trimmed` is taken
        // as-is. Exactly one bracket present (a truncated or malformed
        // coded-URL) is rejected rather than silently kept, so a corrupted
        // server header cannot be echoed back verbatim in a later `If` or
        // `Lock-Token` header.
        let token = match (trimmed.starts_with('<'), trimmed.ends_with('>')) {
            (true, true) => trimmed[1..trimmed.len() - 1].trim(),
            (false, false) => trimmed,
            _ => return Err(invalid()),
        };
        if token.is_empty() {
            return Err(invalid());
        }
        Ok(Self(token.to_owned()))
    }
}

/// `DAV` response header ([RFC 4918 section 10.1](https://www.rfc-editor.org/rfc/rfc4918#section-10.1)): the compliance classes a server advertises.
///
/// # Examples
///
/// ```
/// use dav_xml_client::headers::DavHeader;
///
/// let dav: DavHeader = "1, 2, 3, extended-mkcol".parse().unwrap();
/// assert!(dav.supports_locking());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DavHeader {
    /// The compliance classes, e.g. `"1"`, `"2"`, `"3"`, `"extended-mkcol"`.
    pub classes: Vec<String>,
}

impl DavHeader {
    /// Whether class `2` (locking) is advertised.
    #[must_use]
    pub fn supports_locking(&self) -> bool {
        self.classes.iter().any(|class| class == "2")
    }
}

impl FromStr for DavHeader {
    type Err = InvalidHeader;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            classes: s
                .split(',')
                .map(str::trim)
                .filter(|class| !class.is_empty())
                .map(str::to_owned)
                .collect(),
        })
    }
}

/// `Timeout` request header ([RFC 4918 section 10.7](https://www.rfc-editor.org/rfc/rfc4918#section-10.7)): one or more requested timeouts, in decreasing order of preference.
///
/// # Examples
///
/// ```
/// use dav_xml::elements::Timeout;
/// use dav_xml_client::headers::TimeoutHeader;
///
/// let header = TimeoutHeader(vec![Timeout::Seconds(600), Timeout::Infinite]);
/// assert_eq!(header.to_string(), "Second-600, Infinite");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeoutHeader(
    /// The requested timeouts, in decreasing order of preference.
    pub Vec<Timeout>,
);

impl Display for TimeoutHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, timeout) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{timeout}")?;
        }
        Ok(())
    }
}

impl FromStr for TimeoutHeader {
    type Err = InvalidHeader;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let timeouts = s
            .split(',')
            .map(str::trim)
            .map(|segment| {
                segment.parse::<Timeout>().map_err(|_error| InvalidHeader {
                    header: "Timeout",
                    value: s.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(timeouts))
    }
}
