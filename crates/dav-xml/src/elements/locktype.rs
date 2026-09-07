// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_write() {
        let locktype =
            LockType::from_xml(br#"<D:locktype xmlns:D="DAV:"><D:write/></D:locktype>"#.to_vec())
                .unwrap();
        assert_eq!(locktype, LockType::Write);
    }

    #[test]
    fn rejects_missing_write() {
        let error = LockType::from_xml(br#"<D:locktype xmlns:D="DAV:"></D:locktype>"#.to_vec())
            .unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "locktype",
                element: "write"
            }
        ));
    }

    #[test]
    fn round_trips() {
        let xml = LockType::Write.into_xml().unwrap();
        assert_eq!(LockType::from_xml(xml).unwrap(), LockType::Write);
    }
}

use crate::elements::Write;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `locktype` XML element from RFC 4918 section 14.14.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LockType {
    /// The lock applies to write operations.
    Write,
}

impl Element for LockType {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "locktype";
}

impl TryFrom<&Value> for LockType {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        map.get_required::<Self, Write>()?;
        Ok(Self::Write)
    }
}

impl From<LockType> for Value {
    fn from(_: LockType) -> Value {
        let mut map = ValueMap::new();
        map.insert::<Write>(Value::Empty);
        Value::Map(map)
    }
}
