// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `collection` XML element as defined in
/// [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_collection).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Collection;

impl Element for Collection {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "collection";
}

impl TryFrom<&Value> for Collection {
    type Error = Error;

    fn try_from(_: &Value) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl From<Collection> for Value {
    fn from(_: Collection) -> Value {
        Value::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let collection =
            Collection::from_xml(br#"<D:collection xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        let output = collection.into_xml().unwrap();
        assert_eq!(Collection::from_xml(output).unwrap(), Collection);
    }
}
