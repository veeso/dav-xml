// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::Prop;
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `set` XML element as defined in [RFC 4918 section 14.26](https://www.rfc-editor.org/rfc/rfc4918#section-14.26).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::{Prop, Set};
/// use dav_xml::properties::DisplayName;
///
/// let set = Set(Prop::builder().property(DisplayName("Notes".into())).build());
/// assert!(set.0.displayname().is_some());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Set(pub Prop);

impl Element for Set {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "set";
}

impl TryFrom<&Value> for Set {
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

impl From<Set> for Value {
    fn from(Set(prop): Set) -> Self {
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
    fn parses_writes_and_round_trips() {
        let set = Set::from_xml(
            br#"<D:set xmlns:D="DAV:"><D:prop><D:x>1</D:x></D:prop></D:set>"#.to_vec(),
        )
        .unwrap();
        let output = set.clone().into_xml().unwrap();
        assert_eq!(Set::from_xml(output).unwrap(), set);
    }

    #[test]
    fn missing_prop_is_reported() {
        let error = Set::from_xml(br#"<D:set xmlns:D="DAV:"/>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "set",
                element: "prop",
            }
        ));
    }
}
