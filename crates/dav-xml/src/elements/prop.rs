// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bytestring::ByteString;

use crate::element::{Element, ElementName};
use crate::properties::{
    ContentLanguage, ContentLength, ContentType, CreationDate, DisplayName, ETag, LastModified,
    LockDiscovery, ResourceType, SupportedLock,
};
use crate::value::{Value, ValueMap};
use crate::{DAV_NAMESPACE, DAV_PREFIX, Error};

/// The `prop` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_prop).
///
/// This element can contain arbitrary child elements and supports extracting
/// them using [`Prop::get()`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Prop(ValueMap);

impl Prop {
    /// Read a specific property from this `prop` element.
    ///
    /// Returns
    /// - `None` if the property doesn't exist
    /// - `Some(None)` if the property exists and is empty
    /// - `Some(Some(Ok(_)))` if the property exists and was successfully
    ///   extracted
    /// - `Some(Some(Err(_)))` if the property exists and extraction failed
    #[must_use]
    pub fn get<'v, P>(&'v self) -> Option<Option<Result<P, Error>>>
    where
        P: Element + TryFrom<&'v Value, Error = Error>,
    {
        self.0.get_optional()
    }

    /// Insert or replace property `P` with `value`.
    pub fn insert<P: Element>(&mut self, value: Value) {
        self.0.insert::<P>(value);
    }
}

impl Prop {
    /// Read the `creationdate` property.
    ///
    /// See [`Prop::get()`] for an overview of the possible return values.
    #[must_use]
    pub fn creationdate(&self) -> Option<Option<Result<CreationDate, Error>>> {
        self.get()
    }

    /// Read the `getcontentlength` property.
    ///
    /// See [`Prop::get()`] for an overview of the possible return values.
    #[must_use]
    pub fn getcontentlength(&self) -> Option<Option<Result<ContentLength, Error>>> {
        self.get()
    }

    /// Read the `getlastmodified` property.
    ///
    /// See [`Prop::get()`] for an overview of the possible return values.
    #[must_use]
    pub fn getlastmodified(&self) -> Option<Option<Result<LastModified, Error>>> {
        self.get()
    }

    /// Read the `displayname` property.
    #[must_use]
    pub fn displayname(&self) -> Option<Option<Result<DisplayName, Error>>> {
        self.get()
    }

    /// Read the `getcontentlanguage` property.
    #[must_use]
    pub fn getcontentlanguage(&self) -> Option<Option<Result<ContentLanguage, Error>>> {
        self.get()
    }

    /// Read the `getcontenttype` property.
    #[must_use]
    pub fn getcontenttype(&self) -> Option<Option<Result<ContentType, Error>>> {
        self.get()
    }

    /// Read the `getetag` property.
    #[must_use]
    pub fn getetag(&self) -> Option<Option<Result<ETag, Error>>> {
        self.get()
    }

    /// Read the `resourcetype` property.
    #[must_use]
    pub fn resourcetype(&self) -> Option<Option<Result<ResourceType, Error>>> {
        self.get()
    }

    /// Read the `lockdiscovery` property.
    #[must_use]
    pub fn lockdiscovery(&self) -> Option<Option<Result<LockDiscovery, Error>>> {
        self.get()
    }

    /// Read the `supportedlock` property.
    #[must_use]
    pub fn supportedlock(&self) -> Option<Option<Result<SupportedLock, Error>>> {
        self.get()
    }

    /// Names of every child element, including custom properties.
    pub fn names(&self) -> impl Iterator<Item = &ElementName<ByteString>> {
        self.0.iter().map(|(name, _)| name)
    }
}

impl Element for Prop {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "prop";
}

impl TryFrom<&Value> for Prop {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Empty => Ok(Self::default()),
            _ => value.as_map_of::<Self>().cloned().map(Self),
        }
    }
}

impl From<Prop> for Value {
    fn from(Prop(map): Prop) -> Value {
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::elements::{Condition, DavError, LockScope, LockType};
    use crate::{FromXml, IntoXml};

    const APACHE: &str = r#"<D:prop xmlns:D="DAV:" xmlns:lp1="DAV:" xmlns:lp2="http://apache.org/dav/props/">
  <lp1:resourcetype><D:collection/></lp1:resourcetype>
  <lp1:creationdate>2024-01-01T00:00:00Z</lp1:creationdate>
  <lp1:getlastmodified>Mon, 01 Jan 2024 00:00:00 GMT</lp1:getlastmodified>
  <lp1:getetag>"abc"</lp1:getetag>
  <lp1:getcontentlength>12</lp1:getcontentlength>
  <D:getcontenttype>text/plain</D:getcontenttype>
  <D:displayname>x</D:displayname>
  <D:getcontentlanguage>en</D:getcontentlanguage>
  <lp2:executable>F</lp2:executable>
  <D:lockdiscovery/>
  <D:supportedlock/>
</D:prop>"#;

    #[test]
    fn typed_getters_read_every_property() {
        let prop = Prop::from_xml(APACHE.as_bytes().to_vec()).unwrap();
        assert!(
            prop.resourcetype()
                .unwrap()
                .unwrap()
                .unwrap()
                .is_collection()
        );
        prop.creationdate().unwrap().unwrap().unwrap();
        prop.getlastmodified().unwrap().unwrap().unwrap();
        assert_eq!(prop.getetag().unwrap().unwrap().unwrap().tag, "abc");
        assert_eq!(prop.getcontentlength().unwrap().unwrap().unwrap().0, 12);
        assert_eq!(
            prop.getcontenttype().unwrap().unwrap().unwrap().0,
            mime::TEXT_PLAIN
        );
        assert_eq!(prop.displayname().unwrap().unwrap().unwrap().0, "x");
        assert_eq!(prop.getcontentlanguage().unwrap().unwrap().unwrap().0, "en");
        assert!(matches!(prop.lockdiscovery(), Some(None)));
        assert!(matches!(prop.supportedlock(), Some(None)));
    }

    #[test]
    fn absent_is_none_and_empty_is_some_none() {
        let prop = Prop::from_xml(br#"<D:prop xmlns:D="DAV:"><D:displayname/></D:prop>"#.to_vec())
            .unwrap();
        assert!(matches!(prop.displayname(), Some(None)));
        assert!(prop.getetag().is_none());
    }

    #[test]
    fn typed_getters_read_populated_lock_properties() {
        let prop = Prop::from_xml(
            br#"<D:prop xmlns:D="DAV:">
  <D:lockdiscovery><D:activelock><D:lockscope><D:shared/></D:lockscope><D:locktype><D:write/></D:locktype><D:depth>0</D:depth><D:lockroot><D:href>/locked</D:href></D:lockroot></D:activelock></D:lockdiscovery>
  <D:supportedlock><D:lockentry><D:lockscope><D:exclusive/></D:lockscope><D:locktype><D:write/></D:locktype></D:lockentry></D:supportedlock>
</D:prop>"#
                .to_vec(),
        )
        .unwrap();
        let lock_discovery = prop.lockdiscovery().unwrap().unwrap().unwrap();
        let supported_lock = prop.supportedlock().unwrap().unwrap().unwrap();

        assert_eq!(lock_discovery.0.len(), 1);
        assert_eq!(lock_discovery.0[0].lockroot.0.path(), "/locked");
        assert!(supported_lock.supports(LockScope::Exclusive, LockType::Write));
    }

    #[test]
    fn names_lists_every_child_including_foreign() {
        let prop = Prop::from_xml(APACHE.as_bytes().to_vec()).unwrap();
        assert_eq!(prop.names().count(), 11);
        assert!(prop.names().any(|name| name.local_name == "executable"));
    }

    #[test]
    fn rejects_nested_empty_lock_token_submitted() {
        let mut prop = Prop::default();
        prop.insert::<DavError>(DavError::single(Condition::LockTokenSubmitted(Vec::new())).into());

        let error = prop.into_xml().unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "lock-token-submitted",
                element: "href",
            }
        ));
    }

    #[test]
    fn serializes_nested_empty_no_conflicting_lock() {
        let mut prop = Prop::default();
        prop.insert::<DavError>(DavError::single(Condition::NoConflictingLock(Vec::new())).into());

        prop.into_xml().unwrap();
    }
}
