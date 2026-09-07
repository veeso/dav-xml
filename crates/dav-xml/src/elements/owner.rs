// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bytestring::ByteString;

use crate::elements::Href;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `owner` XML element
/// ([RFC 4918 section 14.17](https://www.rfc-editor.org/rfc/rfc4918#section-14.17)).
///
/// The content is opaque to the protocol; it is kept as a raw [`Value`].
///
/// # Examples
///
/// ```
/// use dav_xml::elements::Owner;
///
/// let owner = Owner::text("Jane");
/// assert_eq!(owner.as_text(), Some("Jane"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Owner(pub Value);

impl Owner {
    /// An owner identified by text.
    #[must_use]
    pub fn text(text: impl Into<ByteString>) -> Self {
        Self(Value::Text(text.into()))
    }

    /// An owner identified by an `href` child.
    #[must_use]
    pub fn href(href: Href) -> Self {
        let mut map = ValueMap::new();
        map.insert::<Href>(href.into());
        Self(Value::Map(map))
    }

    /// The text content, if the owner is plain text.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match &self.0 {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The `href` child, if present and valid.
    #[must_use]
    pub fn as_href(&self) -> Option<Href> {
        self.0.as_map().ok()?.get::<Href>()?.ok()
    }
}

impl Element for Owner {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "owner";
}

impl TryFrom<&Value> for Owner {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        Ok(Self(value.clone()))
    }
}

impl From<Owner> for Value {
    fn from(Owner(value): Owner) -> Value {
        value
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_href_owner() {
        let xml = br#"<D:owner xmlns:D="DAV:"><D:href>http://example.org/~ejw/contact.html</D:href></D:owner>"#;
        let owner = Owner::from_xml(xml.to_vec()).unwrap();
        assert_eq!(owner.as_href().unwrap().host(), Some("example.org"));
    }

    #[test]
    fn parses_text_owner() {
        let owner = Owner::from_xml(br#"<D:owner xmlns:D="DAV:">Jane</D:owner>"#.to_vec()).unwrap();
        assert_eq!(owner.as_text(), Some("Jane"));
        assert!(owner.as_href().is_none());
    }

    #[test]
    fn parses_empty_owner() {
        let owner = Owner::from_xml(br#"<D:owner xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        assert_eq!(owner, Owner(Value::Empty));
    }

    #[test]
    fn round_trips_constructors() {
        for owner in [Owner::text("Jane"), Owner::href("/u".parse().unwrap())] {
            let xml = owner.clone().into_xml().unwrap();
            assert_eq!(Owner::from_xml(xml).unwrap(), owner);
        }
    }
}
