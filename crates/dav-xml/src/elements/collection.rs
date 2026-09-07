// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// The `collection` XML element
/// ([RFC 4918 section 14.3](https://www.rfc-editor.org/rfc/rfc4918#section-14.3)).
///
/// Identifies a resource as a collection. Extension child elements are
/// accepted, as permitted by RFC 4918, but are not represented by this unit
/// type.
///
/// # Examples
///
/// ```
/// use dav_xml::FromXml;
/// use dav_xml::elements::Collection;
///
/// let collection =
///     Collection::from_xml(br#"<D:collection xmlns:D="DAV:"/>"#.to_vec()).unwrap();
/// assert_eq!(collection, Collection);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Collection;

impl crate::Element for Collection {
    const NAMESPACE: &'static str = crate::DAV_NAMESPACE;
    const PREFIX: &'static str = crate::DAV_PREFIX;
    const LOCAL_NAME: &'static str = "collection";
}

impl TryFrom<&crate::Value> for Collection {
    type Error = crate::Error;

    fn try_from(value: &crate::Value) -> Result<Self, Self::Error> {
        match value {
            crate::Value::Empty | crate::Value::Map(_) => Ok(Self),
            _ => Err(crate::Error::InvalidValueType {
                element: <Self as crate::Element>::LOCAL_NAME,
                expected: "an empty element or element extensions",
            }),
        }
    }
}

impl From<Collection> for crate::Value {
    fn from(_: Collection) -> Self {
        crate::Value::Empty
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let collection =
            Collection::from_xml(br#"<D:collection xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        let output = collection.into_xml().unwrap();
        assert_eq!(Collection::from_xml(output).unwrap(), Collection);
    }

    #[test]
    fn accepts_extension_children() {
        let collection = Collection::from_xml(
            br#"<D:collection xmlns:D="DAV:" xmlns:Z="urn:example">
  <Z:extension/>
</D:collection>"#
                .to_vec(),
        )
        .unwrap();

        assert_eq!(collection, Collection);
    }
}
