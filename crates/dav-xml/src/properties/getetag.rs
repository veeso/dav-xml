// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Display;
use std::str::FromStr;

use bytestring::ByteString;

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `getetag` property as defined in RFC 4918 section 15.6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ETag {
    /// Whether the tag is a weak validator (`W/"..."`).
    pub weak: bool,
    /// The opaque tag without quotes.
    pub tag: ByteString,
}

impl ETag {
    /// Construct a strong entity tag.
    #[must_use]
    pub fn strong(tag: impl Into<ByteString>) -> Self {
        Self {
            weak: false,
            tag: tag.into(),
        }
    }

    /// Construct a weak entity tag.
    #[must_use]
    pub fn weak(tag: impl Into<ByteString>) -> Self {
        Self {
            weak: true,
            tag: tag.into(),
        }
    }
}

impl Element for ETag {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "getetag";
}

impl TryFrom<&Value> for ETag {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        Ok(match value.as_str_of::<Self>()?.parse::<Self>() {
            Ok(etag) => etag,
            Err(error) => match error {},
        })
    }
}

impl From<ETag> for Value {
    fn from(etag: ETag) -> Value {
        Value::from(etag.to_string())
    }
}

impl FromStr for ETag {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (weak, rest) = match s.strip_prefix("W/") {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        let tag = rest
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(rest);
        Ok(Self {
            weak,
            tag: tag.to_owned().into(),
        })
    }
}

impl Display for ETag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.weak {
            f.write_str("W/")?;
        }
        write!(f, "\"{tag}\"", tag = self.tag)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn parses_strong_and_weak() {
        let strong = ETag::try_from(&Value::Text("\"abc\"".into())).unwrap();
        assert_eq!(strong, ETag::strong("abc"));
        let weak = ETag::try_from(&Value::Text("W/\"abc\"".into())).unwrap();
        assert_eq!(weak, ETag::weak("abc"));
    }

    #[test]
    fn parses_unquoted_leniently() {
        let etag = ETag::try_from(&Value::Text("abc".into())).unwrap();
        assert_eq!(etag, ETag::strong("abc"));
    }

    #[test]
    fn displays_quoted() {
        assert_eq!(ETag::strong("x").to_string(), "\"x\"");
        assert_eq!(ETag::weak("x").to_string(), "W/\"x\"");
    }

    #[test]
    fn round_trips() {
        let value: Value = ETag::weak("q").into();
        assert_eq!(ETag::try_from(&value).unwrap(), ETag::weak("q"));
    }
}
