// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{LockScope, LockType, Owner};
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `lockinfo` XML element
/// ([RFC 4918 section 14.16](https://www.rfc-editor.org/rfc/rfc4918#section-14.16)).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::LockInfo;
/// use dav_xml::IntoXml;
///
/// let info = LockInfo::exclusive_write();
/// let xml = info.into_xml().unwrap();
/// assert!(std::str::from_utf8(&xml).unwrap().contains("<D:lockinfo"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LockInfo {
    /// The requested scope of the lock.
    pub lockscope: LockScope,
    /// The requested type of the lock.
    pub locktype: LockType,
    /// The optional owner of the lock.
    pub owner: Option<Owner>,
}

impl LockInfo {
    /// An exclusive write lock request without owner.
    #[must_use]
    pub fn exclusive_write() -> Self {
        Self {
            lockscope: LockScope::Exclusive,
            locktype: LockType::Write,
            owner: None,
        }
    }

    /// A shared write lock request without owner.
    #[must_use]
    pub fn shared_write() -> Self {
        Self {
            lockscope: LockScope::Shared,
            locktype: LockType::Write,
            owner: None,
        }
    }

    /// Attach an owner.
    #[must_use]
    pub fn with_owner(mut self, owner: Owner) -> Self {
        self.owner = Some(owner);
        self
    }
}

impl Element for LockInfo {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "lockinfo";
}

impl TryFrom<&Value> for LockInfo {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        Ok(Self {
            lockscope: map.get_required::<Self, LockScope>()?,
            locktype: map.get_required::<Self, LockType>()?,
            owner: map.get().transpose()?,
        })
    }
}

impl From<LockInfo> for Value {
    fn from(info: LockInfo) -> Value {
        let mut map = ValueMap::new();
        map.insert::<LockScope>(info.lockscope.into());
        map.insert::<LockType>(info.locktype.into());
        if let Some(owner) = info.owner {
            map.insert::<Owner>(owner.into());
        }
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    const RFC_9_10_7: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:lockinfo xmlns:D='DAV:'>
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:locktype><D:write/></D:locktype>
  <D:owner>
    <D:href>http://example.org/~ejw/contact.html</D:href>
  </D:owner>
</D:lockinfo>"#;

    #[test]
    fn parses_rfc_example() {
        let info = LockInfo::from_xml(RFC_9_10_7.as_bytes().to_vec()).unwrap();
        assert_eq!(info.lockscope, LockScope::Exclusive);
        assert_eq!(info.locktype, LockType::Write);
        assert!(info.owner.unwrap().as_href().is_some());
    }

    #[test]
    fn owner_is_optional() {
        let xml = br#"<D:lockinfo xmlns:D="DAV:"><D:lockscope><D:shared/></D:lockscope><D:locktype><D:write/></D:locktype></D:lockinfo>"#;
        let info = LockInfo::from_xml(xml.to_vec()).unwrap();
        assert_eq!(info, LockInfo::shared_write());
    }

    #[test]
    fn writes_request_body() {
        let info = LockInfo::exclusive_write().with_owner(Owner::href("/me".parse().unwrap()));
        let xml = info.clone().into_xml().unwrap();
        let text = std::str::from_utf8(&xml).unwrap();
        assert!(text.contains("<D:lockinfo xmlns:D=\"DAV:\">"), "{text}");
        assert!(text.contains("<D:exclusive/>"), "{text}");
        assert!(text.contains("<D:href>/me</D:href>"), "{text}");
        assert_eq!(LockInfo::from_xml(xml).unwrap(), info);
    }
}
