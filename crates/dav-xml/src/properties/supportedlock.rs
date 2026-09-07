// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{LockEntry, LockScope, LockType};
use crate::value::list_value;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `supportedlock` property
/// ([RFC 4918 section 15.10](https://www.rfc-editor.org/rfc/rfc4918#section-15.10)).
#[derive(Clone, Debug, Default, PartialEq)]
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
    use crate::FromXml;

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
}
