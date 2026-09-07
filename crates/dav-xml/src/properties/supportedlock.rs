// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{LockEntry, LockScope, LockType};
use crate::value::list_value;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `supportedlock` property
/// ([RFC 4918 section 15.10](https://www.rfc-editor.org/rfc/rfc4918#section-15.10)).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::{LockScope, LockType};
/// use dav_xml::properties::SupportedLock;
/// use dav_xml::FromXml;
///
/// let locks = SupportedLock::from_xml(br#"<D:supportedlock xmlns:D="DAV:"><D:lockentry><D:lockscope><D:shared/></D:lockscope><D:locktype><D:write/></D:locktype></D:lockentry></D:supportedlock>"#.to_vec()).unwrap();
/// assert!(locks.supports(LockScope::Shared, LockType::Write));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SupportedLock(pub Vec<LockEntry>);

impl SupportedLock {
    /// Whether a lock with `scope` and `kind` is advertised.
    #[must_use]
    pub fn supports(&self, scope: LockScope, kind: LockType) -> bool {
        self.0
            .iter()
            .any(|entry| entry.lockscope == scope && entry.locktype == kind)
    }
}

impl Element for SupportedLock {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "supportedlock";
}

impl TryFrom<&Value> for SupportedLock {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Empty => Ok(Self::default()),
            other => other.as_map_of::<Self>()?.get_all::<LockEntry>().map(Self),
        }
    }
}

impl From<SupportedLock> for Value {
    fn from(SupportedLock(entries): SupportedLock) -> Value {
        let value = list_value(entries);
        if value == Value::Empty {
            return Value::Empty;
        }

        let mut map = ValueMap::new();
        map.insert::<LockEntry>(value);
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    const RFC_15_10: &str = r#"<D:supportedlock xmlns:D="DAV:">
  <D:lockentry><D:lockscope><D:exclusive/></D:lockscope><D:locktype><D:write/></D:locktype></D:lockentry>
  <D:lockentry><D:lockscope><D:shared/></D:lockscope><D:locktype><D:write/></D:locktype></D:lockentry>
</D:supportedlock>"#;

    #[test]
    fn parses_rfc_example() {
        let sl = SupportedLock::from_xml(RFC_15_10.as_bytes().to_vec()).unwrap();
        assert_eq!(sl.0.len(), 2);
        assert!(sl.supports(LockScope::Exclusive, LockType::Write));
        assert!(sl.supports(LockScope::Shared, LockType::Write));
    }

    #[test]
    fn empty_supports_nothing() {
        let sl = SupportedLock::try_from(&Value::Empty).unwrap();
        assert!(!sl.supports(LockScope::Exclusive, LockType::Write));
    }

    #[test]
    fn writes_empty_element() {
        let xml = String::from_utf8(SupportedLock::default().into_xml().unwrap().to_vec()).unwrap();
        assert!(xml.ends_with(r#"<D:supportedlock xmlns:D="DAV:"/>"#));
    }

    #[test]
    fn round_trips_one_lock_entry() {
        let supported_lock = SupportedLock(vec![LockEntry {
            lockscope: LockScope::Exclusive,
            locktype: LockType::Write,
        }]);
        let xml = supported_lock.clone().into_xml().unwrap();
        assert_eq!(SupportedLock::from_xml(xml).unwrap(), supported_lock);
    }

    #[test]
    fn round_trips_multiple_lock_entries() {
        let supported_lock = SupportedLock(vec![
            LockEntry {
                lockscope: LockScope::Exclusive,
                locktype: LockType::Write,
            },
            LockEntry {
                lockscope: LockScope::Shared,
                locktype: LockType::Write,
            },
        ]);
        let xml = supported_lock.clone().into_xml().unwrap();
        assert_eq!(SupportedLock::from_xml(xml).unwrap(), supported_lock);
    }

    #[test]
    fn implements_eq() {
        fn assert_eq<T: Eq>() {}

        assert_eq::<SupportedLock>();
    }
}
