// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::Prop;
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `remove` XML element as defined in [RFC 4918 section 14.23](https://www.rfc-editor.org/rfc/rfc4918#section-14.23).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::{Prop, Remove};
/// use dav_xml::properties::DisplayName;
///
/// let remove = Remove(Prop::builder().name::<DisplayName>().build());
/// assert!(matches!(remove.0.displayname(), Some(None)));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Remove(pub Prop);

impl Element for Remove {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "remove";
}

impl TryFrom<&Value> for Remove {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        if value == &Value::Empty {
            return Err(Error::MissingElement {
                parent: Self::LOCAL_NAME,
                element: Prop::LOCAL_NAME,
            });
        }
        value
            .as_map_of::<Self>()?
            .get_required::<Self, Prop>()
            .map(Self)
    }
}

impl From<Remove> for Value {
    fn from(Remove(prop): Remove) -> Self {
        let prop = prop
            .names()
            .fold(Prop::builder(), |builder, name| {
                builder.raw(name.clone(), Value::Empty)
            })
            .build();
        let mut map = ValueMap::new();
        map.insert::<Prop>(prop.into());
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips_empty_property_names() {
        let remove = Remove::from_xml(
            br#"<D:remove xmlns:D="DAV:"><D:prop><D:x/></D:prop></D:remove>"#.to_vec(),
        )
        .unwrap();
        let output = remove.clone().into_xml().unwrap();
        assert_eq!(Remove::from_xml(output).unwrap(), remove);
    }

    #[test]
    fn serializes_populated_prop_children_as_empty_names() {
        let remove = Remove::from_xml(
            br#"<D:remove xmlns:D="DAV:"><D:prop><D:x>1</D:x></D:prop></D:remove>"#.to_vec(),
        )
        .unwrap();
        let output = remove.into_xml().unwrap();
        let expected = Remove::from_xml(
            br#"<D:remove xmlns:D="DAV:"><D:prop><D:x/></D:prop></D:remove>"#.to_vec(),
        )
        .unwrap();

        assert_eq!(Remove::from_xml(output).unwrap(), expected);
    }

    #[test]
    fn missing_prop_is_reported() {
        let error = Remove::from_xml(br#"<D:remove xmlns:D="DAV:"/>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "remove",
                element: "prop",
            }
        ));
    }
}
