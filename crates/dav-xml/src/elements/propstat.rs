// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{DavError, Prop, ResponseDescription, Status};
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `propstat` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_propstat).
#[derive(Clone, Debug, PartialEq)]
pub struct Propstat {
    /// The properties to which the status applies.
    pub prop: Prop,
    /// The status for those properties.
    pub status: Status,
    /// An optional error describing failed properties.
    pub error: Option<DavError>,
    /// An optional human-readable description.
    pub responsedescription: Option<ResponseDescription>,
}

impl Element for Propstat {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "propstat";
}

impl Propstat {
    /// Creates a property status without an error or response description.
    #[must_use]
    pub fn new(prop: Prop, status: Status) -> Self {
        Self {
            prop,
            status,
            error: None,
            responsedescription: None,
        }
    }
}

impl TryFrom<&Value> for Propstat {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        Ok(Self {
            prop: map.get_required::<Self, Prop>()?,
            status: map.get_required::<Self, Status>()?,
            error: map.get().transpose()?,
            responsedescription: map.get().transpose()?,
        })
    }
}

impl From<Propstat> for Value {
    fn from(
        Propstat {
            prop,
            status,
            error,
            responsedescription,
        }: Propstat,
    ) -> Value {
        let mut map = ValueMap::new();

        map.insert::<Prop>(prop.into());
        map.insert::<Status>(status.into());
        if let Some(error) = error {
            map.insert::<DavError>(error.into());
        }
        if let Some(responsedescription) = responsedescription {
            map.insert::<ResponseDescription>(responsedescription.into());
        }

        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let xml = r#"<D:propstat xmlns:D="DAV:"><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat>"#;
        let propstat = Propstat::from_xml(xml.as_bytes().to_vec()).unwrap();
        let output = propstat.clone().into_xml().unwrap();
        assert_eq!(Propstat::from_xml(output).unwrap(), propstat);
    }

    #[test]
    fn parses_error_child() {
        let xml = br#"<D:propstat xmlns:D="DAV:"><D:prop><D:x/></D:prop><D:status>HTTP/1.1 403 Forbidden</D:status><D:error><D:cannot-modify-protected-property/></D:error></D:propstat>"#;
        let propstat = Propstat::from_xml(xml.to_vec()).unwrap();
        assert!(
            propstat
                .error
                .as_ref()
                .unwrap()
                .contains("cannot-modify-protected-property")
        );
        assert_eq!(
            Propstat::from_xml(propstat.clone().into_xml().unwrap()).unwrap(),
            propstat
        );
    }

    #[test]
    fn new_has_no_optional_children() {
        let propstat = Propstat::new(Prop::default(), Status::new(http::StatusCode::OK));
        assert!(propstat.error.is_none());
        assert!(propstat.responsedescription.is_none());
    }
}
