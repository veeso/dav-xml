// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

pub use crate::elements::Collection;
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `resourcetype` property as defined in
/// [RFC 4918](http://webdav.org/specs/rfc4918.html#PROPERTY_resourcetype).
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceType(ValueMap);

impl ResourceType {
    /// Construct a plain, non-collection resource type.
    #[must_use]
    pub fn resource() -> Self {
        Self(ValueMap::new())
    }

    /// Construct a collection resource type.
    #[must_use]
    pub fn collection() -> Self {
        let mut map = ValueMap::new();
        map.insert::<Collection>(Value::Empty);
        Self(map)
    }

    /// Whether this resource type contains a `DAV:collection` child.
    #[must_use]
    pub fn is_collection(&self) -> bool {
        self.0.get::<Collection>().is_some()
    }
}

impl Element for ResourceType {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "resourcetype";
}

impl TryFrom<&Value> for ResourceType {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        if value == &Value::Empty {
            return Ok(Self::resource());
        }
        value.as_map_of::<Self>().cloned().map(Self)
    }
}

impl From<ResourceType> for Value {
    fn from(ResourceType(map): ResourceType) -> Value {
        if map.is_empty() {
            Value::Empty
        } else {
            Value::Map(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FromXml;

    #[test]
    fn empty_is_not_collection() {
        let resource_type = ResourceType::try_from(&Value::Empty).unwrap();
        assert!(!resource_type.is_collection());
    }

    #[test]
    fn collection_child_is_collection() {
        let resource_type = ResourceType::from_xml(
            br#"<D:resourcetype xmlns:D="DAV:"><D:collection/></D:resourcetype>"#.to_vec(),
        )
        .unwrap();
        assert!(resource_type.is_collection());
    }

    #[test]
    fn foreign_child_is_kept() {
        let resource_type = ResourceType::from_xml(
            br#"<D:resourcetype xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"><D:collection/><C:calendar/></D:resourcetype>"#
                .to_vec(),
        )
        .unwrap();
        assert!(resource_type.is_collection());
        assert_eq!(resource_type.0.len(), 2);
    }

    #[test]
    fn constructors() {
        assert!(ResourceType::collection().is_collection());
        assert!(!ResourceType::resource().is_collection());
        let value: Value = ResourceType::resource().into();
        assert_eq!(value, Value::Empty);
    }
}
