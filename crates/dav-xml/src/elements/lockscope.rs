// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_exclusive_and_shared() {
        let e = LockScope::from_xml(
            br#"<D:lockscope xmlns:D="DAV:"><D:exclusive/></D:lockscope>"#.to_vec(),
        )
        .unwrap();
        assert_eq!(e, LockScope::Exclusive);
        let s = LockScope::from_xml(
            br#"<D:lockscope xmlns:D="DAV:"><D:shared/></D:lockscope>"#.to_vec(),
        )
        .unwrap();
        assert_eq!(s, LockScope::Shared);
    }

    #[test]
    fn rejects_both_or_neither() {
        let both = LockScope::from_xml(
            br#"<D:lockscope xmlns:D="DAV:"><D:shared/><D:exclusive/></D:lockscope>"#.to_vec(),
        )
        .unwrap_err();
        assert!(matches!(
            both,
            Error::ConflictingElements {
                parent: "lockscope",
                ..
            }
        ));
        let none =
            LockScope::from_xml(br#"<D:lockscope xmlns:D="DAV:"><D:x/></D:lockscope>"#.to_vec())
                .unwrap_err();
        assert!(matches!(
            none,
            Error::MissingElement {
                parent: "lockscope",
                ..
            }
        ));
    }

    #[test]
    fn round_trips() {
        for scope in [LockScope::Exclusive, LockScope::Shared] {
            let xml = scope.into_xml().unwrap();
            assert_eq!(LockScope::from_xml(xml).unwrap(), scope);
        }
    }
}

use crate::elements::{Exclusive, Shared};
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `lockscope` XML element from RFC 4918 section 14.13.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LockScope {
    /// Only one principal may hold the lock.
    Exclusive,
    /// Several principals may hold the lock.
    Shared,
}

impl Element for LockScope {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "lockscope";
}

impl TryFrom<&Value> for LockScope {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        match (map.get::<Exclusive>(), map.get::<Shared>()) {
            (Some(exclusive), None) => exclusive.map(|_| Self::Exclusive),
            (None, Some(shared)) => shared.map(|_| Self::Shared),
            (Some(_), Some(_)) => Err(Error::ConflictingElements {
                parent: Self::LOCAL_NAME,
                elements: "exclusive and shared are mutually exclusive",
            }),
            (None, None) => Err(Error::MissingElement {
                parent: Self::LOCAL_NAME,
                element: "exclusive or shared",
            }),
        }
    }
}

impl From<LockScope> for Value {
    fn from(scope: LockScope) -> Value {
        let mut map = ValueMap::new();
        match scope {
            LockScope::Exclusive => map.insert::<Exclusive>(Value::Empty),
            LockScope::Shared => map.insert::<Shared>(Value::Empty),
        }
        Value::Map(map)
    }
}
