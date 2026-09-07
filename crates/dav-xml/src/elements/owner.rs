// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bytestring::ByteString;

use crate::elements::Href;
use crate::{ContentItem, DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `owner` XML element
/// ([RFC 4918 section 14.17](https://www.rfc-editor.org/rfc/rfc4918#section-14.17)).
///
/// The content is opaque to the protocol; it is kept as a raw [`Value`].
/// Mixed text and child elements use [`Value::Mixed`]. Attributes are not
/// retained because the generic [`Value`] model has no attribute storage.
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
        match &self.0 {
            Value::Map(map) => map.get::<Href>()?.ok(),
            Value::Mixed(items) => items.iter().find_map(|item| match item {
                ContentItem::Element { name, value }
                    if name.namespace.as_deref() == Some(DAV_NAMESPACE)
                        && name.local_name == Href::LOCAL_NAME =>
                {
                    Href::try_from(value).ok()
                }
                _ => None,
            }),
            _ => None,
        }
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
    use crate::{ElementName, FromXml, IntoXml};

    fn dav_name(local_name: &str) -> ElementName<ByteString> {
        ElementName {
            namespace: Some(DAV_NAMESPACE.into()),
            prefix: None,
            local_name: local_name.into(),
        }
    }

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
    fn preserves_whitespace_only_owner_text() {
        let owner = Owner::from_xml(
            br#"<D:owner xmlns:D="DAV:">
  </D:owner>"#
                .to_vec(),
        )
        .unwrap();
        assert_eq!(owner, Owner(Value::Text("\n  ".into())));
    }

    #[test]
    fn preserves_whitespace_before_and_after_owner_children() {
        let xml = br#"<D:owner xmlns:D="DAV:">
  <D:href>/u</D:href>
</D:owner>"#;
        assert_eq!(
            Owner::from_xml(xml.to_vec()).unwrap(),
            Owner(Value::Mixed(vec![
                ContentItem::Text("\n  ".into()),
                ContentItem::Element {
                    name: dav_name("href"),
                    value: Value::Text("/u".into()),
                },
                ContentItem::Text("\n".into()),
            ]))
        );
    }

    #[test]
    fn preserves_whitespace_in_nested_owner_descendants() {
        let xml = br#"<D:owner xmlns:D="DAV:">
  <D:wrapper>
    <D:leaf/>
  </D:wrapper>
</D:owner>"#;
        assert_eq!(
            Owner::from_xml(xml.to_vec()).unwrap(),
            Owner(Value::Mixed(vec![
                ContentItem::Text("\n  ".into()),
                ContentItem::Element {
                    name: dav_name("wrapper"),
                    value: Value::Mixed(vec![
                        ContentItem::Text("\n    ".into()),
                        ContentItem::Element {
                            name: dav_name("leaf"),
                            value: Value::Empty,
                        },
                        ContentItem::Text("\n  ".into()),
                    ]),
                },
                ContentItem::Text("\n".into()),
            ]))
        );
    }

    #[test]
    fn accepts_child_then_text_owner_content() {
        let xml = br#"<D:owner xmlns:D="DAV:"><D:href>/u</D:href>tail</D:owner>"#;
        assert_eq!(
            Owner::from_xml(xml.to_vec()).unwrap(),
            Owner(Value::Mixed(vec![
                ContentItem::Element {
                    name: dav_name("href"),
                    value: Value::Text("/u".into()),
                },
                ContentItem::Text("tail".into()),
            ]))
        );
    }

    #[test]
    fn accepts_text_then_child_owner_content() {
        let xml = br#"<D:owner xmlns:D="DAV:">head<D:href>/u</D:href></D:owner>"#;
        assert_eq!(
            Owner::from_xml(xml.to_vec()).unwrap(),
            Owner(Value::Mixed(vec![
                ContentItem::Text("head".into()),
                ContentItem::Element {
                    name: dav_name("href"),
                    value: Value::Text("/u".into()),
                },
            ]))
        );
    }

    #[test]
    fn preserves_nested_owner_content() {
        let xml =
            br#"<D:owner xmlns:D="DAV:"><D:wrapper>before<D:leaf/>after</D:wrapper>tail</D:owner>"#;
        assert_eq!(
            Owner::from_xml(xml.to_vec()).unwrap(),
            Owner(Value::Mixed(vec![
                ContentItem::Element {
                    name: dav_name("wrapper"),
                    value: Value::Mixed(vec![
                        ContentItem::Text("before".into()),
                        ContentItem::Element {
                            name: dav_name("leaf"),
                            value: Value::Empty,
                        },
                        ContentItem::Text("after".into()),
                    ]),
                },
                ContentItem::Text("tail".into()),
            ]))
        );
    }

    #[test]
    fn preserves_entities_and_cdata_in_mixed_content() {
        let xml = br#"<D:owner xmlns:D="DAV:">head &amp; <![CDATA[before]]><D:href>/u</D:href>after</D:owner>"#;
        let owner = Owner::from_xml(xml.to_vec()).unwrap();
        let Value::Mixed(items) = owner.0 else {
            panic!("owner content is not mixed")
        };
        let child_index = items
            .iter()
            .position(|item| matches!(item, ContentItem::Element { .. }))
            .unwrap();
        let before = items[..child_index]
            .iter()
            .filter_map(|item| match item {
                ContentItem::Text(text) => Some(text.to_string()),
                ContentItem::Element { .. } => None,
            })
            .collect::<String>();
        let after = items[child_index + 1..]
            .iter()
            .filter_map(|item| match item {
                ContentItem::Text(text) => Some(text.to_string()),
                ContentItem::Element { .. } => None,
            })
            .collect::<String>();
        assert_eq!(before, "head & before");
        assert_eq!(after, "after");
    }

    #[test]
    fn round_trips_adjacent_text_cdata_and_entity_chunks() {
        let owner = Owner::from_xml(
            br#"<D:owner xmlns:D="DAV:">head &amp; <![CDATA[before]]><D:href>/u</D:href>tail &amp;<![CDATA[ after]]></D:owner>"#
                .to_vec(),
        )
        .unwrap();

        let xml = owner.clone().into_xml().unwrap();

        assert_eq!(Owner::from_xml(xml).unwrap(), owner);
    }

    #[test]
    fn round_trips_mixed_owner_with_foreign_namespace() {
        let owner = Owner(Value::Mixed(vec![
            ContentItem::Text("before".into()),
            ContentItem::Element {
                name: ElementName {
                    namespace: Some("urn:example".into()),
                    prefix: Some("Z".into()),
                    local_name: "child".into(),
                },
                value: Value::Empty,
            },
            ContentItem::Text("after".into()),
        ]));
        let xml = owner.clone().into_xml().unwrap();
        let xml = std::str::from_utf8(&xml).unwrap();
        assert!(xml.contains("xmlns:Z=\"urn:example\""), "{xml}");
        assert_eq!(Owner::from_xml(xml.as_bytes().to_vec()).unwrap(), owner);
    }

    #[test]
    fn round_trips_mixed_owner_content() {
        for xml in [
            br#"<D:owner xmlns:D="DAV:"><D:href>/u</D:href>tail</D:owner>"#.to_vec(),
            br#"<D:owner xmlns:D="DAV:">head<D:href>/u</D:href></D:owner>"#.to_vec(),
            br#"<D:owner xmlns:D="DAV:"><D:wrapper>before<D:leaf/>after</D:wrapper>tail</D:owner>"#
                .to_vec(),
        ] {
            let owner = Owner::from_xml(xml).unwrap();
            assert_eq!(
                Owner::from_xml(owner.clone().into_xml().unwrap()).unwrap(),
                owner
            );
        }
    }

    #[test]
    fn round_trips_constructors() {
        for owner in [Owner::text("Jane"), Owner::href("/u".parse().unwrap())] {
            let xml = owner.clone().into_xml().unwrap();
            assert_eq!(Owner::from_xml(xml).unwrap(), owner);
        }
    }
}
