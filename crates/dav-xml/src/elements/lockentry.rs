// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{LockScope, LockType};
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `lockentry` XML element
/// ([RFC 4918 section 14.11](https://www.rfc-editor.org/rfc/rfc4918#section-14.11)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LockEntry {
    /// The scope this lock kind supports.
    pub lockscope: LockScope,
    /// The type this lock kind supports.
    pub locktype: LockType,
}

impl Element for LockEntry {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "lockentry";
}

impl TryFrom<&Value> for LockEntry {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        Ok(Self {
            lockscope: map.get_required::<Self, LockScope>()?,
            locktype: map.get_required::<Self, LockType>()?,
        })
    }
}

impl From<LockEntry> for Value {
    fn from(entry: LockEntry) -> Value {
        let mut map = ValueMap::new();
        map.insert::<LockScope>(entry.lockscope.into());
        map.insert::<LockType>(entry.locktype.into());
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_rfc_example() {
        let xml = br#"<D:lockentry xmlns:D="DAV:"><D:lockscope><D:exclusive/></D:lockscope><D:locktype><D:write/></D:locktype></D:lockentry>"#;
        let entry = LockEntry::from_xml(xml.to_vec()).unwrap();
        assert_eq!(
            entry,
            LockEntry {
                lockscope: LockScope::Exclusive,
                locktype: LockType::Write,
            }
        );
    }

    #[test]
    fn missing_locktype_is_reported() {
        let xml =
            br#"<D:lockentry xmlns:D="DAV:"><D:lockscope><D:shared/></D:lockscope></D:lockentry>"#;
        let error = LockEntry::from_xml(xml.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "lockentry",
                element: "locktype"
            }
        ));
    }

    #[test]
    fn round_trips() {
        let entry = LockEntry {
            lockscope: LockScope::Shared,
            locktype: LockType::Write,
        };
        assert_eq!(
            LockEntry::from_xml(entry.into_xml().unwrap()).unwrap(),
            entry
        );
    }
}
