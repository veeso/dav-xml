// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{Prop, ResponseDescription, Status};
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `propstat` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_propstat).
#[derive(Clone, Debug, PartialEq)]
pub struct Propstat {
    /// The properties to which the status applies.
    pub prop: Prop,
    /// The status for those properties.
    pub status: Status,
    // pub error: Option<Error>,
    /// An optional human-readable description.
    pub responsedescription: Option<ResponseDescription>,
}

impl Element for Propstat {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "propstat";
}

impl TryFrom<&Value> for Propstat {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        Ok(Self {
            prop: map.get_required::<Self, Prop>()?,
            status: map.get_required::<Self, Status>()?,
            responsedescription: map.get().transpose()?,
        })
    }
}

impl From<Propstat> for Value {
    fn from(
        Propstat {
            prop,
            status,
            responsedescription,
        }: Propstat,
    ) -> Value {
        let mut map = ValueMap::new();

        map.insert::<Prop>(prop.into());
        map.insert::<Status>(status.into());
        if let Some(responsedescription) = responsedescription {
            map.insert::<ResponseDescription>(responsedescription.into());
        }

        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let xml = r#"<D:propstat xmlns:D="DAV:"><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat>"#;
        let propstat = Propstat::from_xml(xml.as_bytes().to_vec()).unwrap();
        let output = propstat.clone().into_xml().unwrap();
        assert_eq!(Propstat::from_xml(output).unwrap(), propstat);
    }
}
