// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_all_values() {
        for (text, depth) in [
            ("0", Depth::Zero),
            ("1", Depth::One),
            ("infinity", Depth::Infinity),
            ("Infinity", Depth::Infinity),
        ] {
            let xml = format!(r#"<D:depth xmlns:D="DAV:">{text}</D:depth>"#);
            assert_eq!(Depth::from_xml(xml.into_bytes()).unwrap(), depth);
        }
    }

    #[test]
    fn rejects_other_values() {
        let error =
            Depth::from_xml(br#"<D:depth xmlns:D="DAV:">2</D:depth>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidElement {
                element: "depth",
                ..
            }
        ));
    }

    #[test]
    fn displays_header_value() {
        assert_eq!(Depth::Zero.to_string(), "0");
        assert_eq!(Depth::One.to_string(), "1");
        assert_eq!(Depth::Infinity.to_string(), "infinity");
    }

    #[test]
    fn round_trips() {
        for depth in [Depth::Zero, Depth::One, Depth::Infinity] {
            assert_eq!(Depth::from_xml(depth.into_xml().unwrap()).unwrap(), depth);
        }
    }
}

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The depth requested for a `WebDAV` operation ([RFC 4918 section 10.2](https://www.rfc-editor.org/rfc/rfc4918#section-10.2)).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::Depth;
/// use std::str::FromStr;
///
/// assert_eq!(Depth::from_str("infinity").unwrap(), Depth::Infinity);
/// assert_eq!(Depth::One.to_string(), "1");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Depth {
    /// The resource only.
    #[default]
    Zero,
    /// The resource and its direct members.
    One,
    /// The whole subtree.
    Infinity,
}

impl Element for Depth {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "depth";
}

impl FromStr for Depth {
    type Err = InvalidDepth;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "0" => Ok(Self::Zero),
            "1" => Ok(Self::One),
            value if value.eq_ignore_ascii_case("infinity") => Ok(Self::Infinity),
            other => Err(InvalidDepth(other.to_owned())),
        }
    }
}

impl Display for Depth {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Infinity => "infinity",
        })
    }
}

impl TryFrom<&Value> for Depth {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map_err(Error::invalid::<Self>)
    }
}

impl From<Depth> for Value {
    fn from(depth: Depth) -> Value {
        depth.to_string().into()
    }
}

/// A `depth` value other than `0`, `1`, or `infinity` ([RFC 4918 section 10.2](https://www.rfc-editor.org/rfc/rfc4918#section-10.2), [section 14.4](https://www.rfc-editor.org/rfc/rfc4918#section-14.4)).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid depth: {0}")]
pub struct InvalidDepth(String);
