// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::ActiveLock;
use crate::value::list_value;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `lockdiscovery` property
/// ([RFC 4918 section 15.8](https://www.rfc-editor.org/rfc/rfc4918#section-15.8)).
///
/// # Examples
///
/// ```
/// use dav_xml::properties::LockDiscovery;
/// use dav_xml::FromXml;
///
/// let locks = LockDiscovery::from_xml(br#"<D:lockdiscovery xmlns:D="DAV:"/>"#.to_vec()).unwrap();
/// assert!(locks.0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LockDiscovery(pub Vec<ActiveLock>);

impl Element for LockDiscovery {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "lockdiscovery";
}

impl TryFrom<&Value> for LockDiscovery {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Empty => Ok(Self::default()),
            other => other.as_map_of::<Self>()?.get_all::<ActiveLock>().map(Self),
        }
    }
}

impl From<LockDiscovery> for Value {
    fn from(LockDiscovery(locks): LockDiscovery) -> Value {
        let value = list_value(locks);
        if value == Value::Empty {
            return Value::Empty;
        }

        let mut map = ValueMap::new();
        map.insert::<ActiveLock>(value);
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::elements::{Depth, Href, LockRoot, LockScope, LockType};
    use crate::{FromXml, IntoXml};

    fn active_lock(path: &str) -> ActiveLock {
        ActiveLock {
            lockscope: LockScope::Shared,
            locktype: LockType::Write,
            depth: Depth::Zero,
            owner: None,
            timeout: None,
            locktoken: None,
            lockroot: Some(LockRoot::from(path.parse::<Href>().unwrap())),
        }
    }

    #[test]
    fn empty_means_no_locks() {
        let ld = LockDiscovery::try_from(&Value::Empty).unwrap();
        assert!(ld.0.is_empty());
    }

    #[test]
    fn parses_two_active_locks() {
        let xml = br#"<D:lockdiscovery xmlns:D="DAV:">
  <D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:shared/></D:lockscope><D:depth>0</D:depth><D:lockroot><D:href>/a</D:href></D:lockroot></D:activelock>
  <D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:shared/></D:lockscope><D:depth>0</D:depth><D:lockroot><D:href>/a</D:href></D:lockroot></D:activelock>
</D:lockdiscovery>"#;
        let ld = LockDiscovery::from_xml(xml.to_vec()).unwrap();
        assert_eq!(ld.0.len(), 2);
    }

    #[test]
    fn empty_serializes_as_empty_element() {
        let value: Value = LockDiscovery(Vec::new()).into();
        assert_eq!(value, Value::Empty);
    }

    #[test]
    fn writes_empty_element() {
        let xml = String::from_utf8(LockDiscovery::default().into_xml().unwrap().to_vec()).unwrap();
        assert!(xml.ends_with(r#"<D:lockdiscovery xmlns:D="DAV:"/>"#));
    }

    #[test]
    fn round_trips_one_active_lock() {
        let lock_discovery = LockDiscovery(vec![active_lock("/a")]);
        let xml = lock_discovery.clone().into_xml().unwrap();
        assert_eq!(LockDiscovery::from_xml(xml).unwrap(), lock_discovery);
    }

    #[test]
    fn round_trips_multiple_active_locks() {
        let lock_discovery = LockDiscovery(vec![active_lock("/a"), active_lock("/b")]);
        let xml = lock_discovery.clone().into_xml().unwrap();
        assert_eq!(LockDiscovery::from_xml(xml).unwrap(), lock_discovery);
    }

    #[test]
    fn implements_eq() {
        fn assert_eq<T: Eq>() {}

        assert_eq::<LockDiscovery>();
    }
}
