// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{Depth, LockRoot, LockScope, LockToken, LockType, Owner, Timeout};
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `activelock` XML element
/// ([RFC 4918 section 14.1](https://www.rfc-editor.org/rfc/rfc4918#section-14.1)).
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveLock {
    /// The lock's scope.
    pub lockscope: LockScope,
    /// The lock's type.
    pub locktype: LockType,
    /// The lock's depth.
    pub depth: Depth,
    /// The optional lock owner.
    pub owner: Option<Owner>,
    /// The optional lock timeout.
    pub timeout: Option<Timeout>,
    /// The optional lock token.
    pub locktoken: Option<LockToken>,
    /// The locked resource's root URI.
    pub lockroot: LockRoot,
}

impl Element for ActiveLock {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "activelock";
}

impl TryFrom<&Value> for ActiveLock {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        Ok(Self {
            lockscope: map.get_required::<Self, LockScope>()?,
            locktype: map.get_required::<Self, LockType>()?,
            depth: map.get_required::<Self, Depth>()?,
            owner: map.get().transpose()?,
            timeout: map.get().transpose()?,
            locktoken: map.get().transpose()?,
            lockroot: map.get_required::<Self, LockRoot>()?,
        })
    }
}

impl From<ActiveLock> for Value {
    fn from(lock: ActiveLock) -> Value {
        let mut map = ValueMap::new();
        map.insert::<LockType>(lock.locktype.into());
        map.insert::<LockScope>(lock.lockscope.into());
        map.insert::<Depth>(lock.depth.into());
        if let Some(owner) = lock.owner {
            map.insert::<Owner>(owner.into());
        }
        if let Some(timeout) = lock.timeout {
            map.insert::<Timeout>(timeout.into());
        }
        if let Some(locktoken) = lock.locktoken {
            map.insert::<LockToken>(locktoken.into());
        }
        map.insert::<LockRoot>(lock.lockroot.into());
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    const RFC_9_10_8: &str = r#"<D:activelock xmlns:D="DAV:">
  <D:locktype><D:write/></D:locktype>
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:depth>infinity</D:depth>
  <D:owner>
    <D:href>http://example.org/~ejw/contact.html</D:href>
  </D:owner>
  <D:timeout>Second-604800</D:timeout>
  <D:locktoken>
    <D:href>urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4</D:href>
  </D:locktoken>
  <D:lockroot>
    <D:href>http://example.com/workspace/webdav/proposal.doc</D:href>
  </D:lockroot>
</D:activelock>"#;

    #[test]
    fn parses_rfc_example() {
        let lock = ActiveLock::from_xml(RFC_9_10_8.as_bytes().to_vec()).unwrap();
        assert_eq!(lock.lockscope, LockScope::Exclusive);
        assert_eq!(lock.locktype, LockType::Write);
        assert_eq!(lock.depth, Depth::Infinity);
        assert_eq!(lock.timeout, Some(Timeout::Seconds(604_800)));
        assert_eq!(
            lock.locktoken.unwrap().0.to_string(),
            "urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4"
        );
        assert_eq!(lock.lockroot.0.path(), "/workspace/webdav/proposal.doc");
    }

    #[test]
    fn optional_children_may_be_absent() {
        let xml = br#"<D:activelock xmlns:D="DAV:"><D:locktype><D:write/></D:locktype><D:lockscope><D:shared/></D:lockscope><D:depth>0</D:depth><D:lockroot><D:href>/x</D:href></D:lockroot></D:activelock>"#;
        let lock = ActiveLock::from_xml(xml.to_vec()).unwrap();
        assert_eq!(lock.owner, None);
        assert_eq!(lock.timeout, None);
        assert_eq!(lock.locktoken, None);
    }

    #[test]
    fn missing_lockroot_is_reported() {
        let xml = br#"<D:activelock xmlns:D="DAV:"><D:locktype><D:write/></D:locktype><D:lockscope><D:shared/></D:lockscope><D:depth>0</D:depth></D:activelock>"#;
        let error = ActiveLock::from_xml(xml.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "activelock",
                element: "lockroot"
            }
        ));
    }

    #[test]
    fn round_trips() {
        let lock = ActiveLock::from_xml(RFC_9_10_8.as_bytes().to_vec()).unwrap();
        assert_eq!(
            ActiveLock::from_xml(lock.clone().into_xml().unwrap()).unwrap(),
            lock
        );
    }
}
