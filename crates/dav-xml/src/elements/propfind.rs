// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::elements::{AllProp, Include, Prop, PropName};
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `propfind` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_propfind).
#[derive(Clone, Debug, PartialEq)]
pub enum PropFind {
    /// Request the names of all properties.
    PropName,
    /// Request all properties, optionally including additional properties.
    AllProp {
        /// Additional properties to include with the standard properties.
        include: Option<Include>,
    },
    /// Request the listed properties.
    Prop(Prop),
}

impl Element for PropFind {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "propfind";
}

impl TryFrom<&Value> for PropFind {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;

        match (
            map.get::<PropName>(),
            map.get::<AllProp>(),
            map.get::<Prop>(),
        ) {
            (Some(_), None, None) => Ok(Self::PropName),
            (None, Some(_), None) => Ok(Self::AllProp {
                include: map.get().transpose()?,
            }),
            (None, None, Some(prop)) => Ok(Self::Prop(prop?)),
            _ => Err(Error::ConflictingElements {
                parent: Self::LOCAL_NAME,
                elements: "propname, allprop and prop are mutually exclusive",
            }),
        }
    }
}
impl From<PropFind> for Value {
    fn from(propfind: PropFind) -> Value {
        let mut map = ValueMap::new();
        match propfind {
            PropFind::PropName => map.insert::<PropName>(Value::Empty),
            PropFind::AllProp { include } => {
                map.insert::<AllProp>(Value::Empty);
                if let Some(include) = include {
                    map.insert::<Include>(include.into());
                }
            }
            PropFind::Prop(prop) => map.insert::<Prop>(prop.into()),
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
    fn parses_propname() {
        let propfind = PropFind::from_xml(
            br#"<D:propfind xmlns:D="DAV:"><D:propname/></D:propfind>"#.to_vec(),
        )
        .unwrap();
        assert_eq!(propfind, PropFind::PropName);
    }

    #[test]
    fn parses_allprop_with_include() {
        let xml = r#"<D:propfind xmlns:D="DAV:"><D:allprop/><D:include><D:supported-live-property-set/></D:include></D:propfind>"#;
        let propfind = PropFind::from_xml(xml.as_bytes().to_vec()).unwrap();
        let PropFind::AllProp {
            include: Some(include),
        } = propfind
        else {
            panic!("expected allprop with include")
        };
        assert_eq!(include.0.len(), 1);
        assert_eq!(include.0[0].local_name, "supported-live-property-set");
    }

    #[test]
    fn parses_prop() {
        let xml = r#"<D:propfind xmlns:D="DAV:"><D:prop><D:displayname/></D:prop></D:propfind>"#;
        let PropFind::Prop(prop) = PropFind::from_xml(xml.as_bytes().to_vec()).unwrap() else {
            panic!("expected prop")
        };
        assert!(prop.get::<crate::properties::DisplayName>().is_some());
    }

    #[test]
    fn rejects_conflicts() {
        let xml = r#"<D:propfind xmlns:D="DAV:"><D:propname/><D:allprop/></D:propfind>"#;
        let error = PropFind::from_xml(xml.as_bytes().to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::ConflictingElements {
                parent: "propfind",
                ..
            }
        ));
    }

    #[test]
    fn writes_allprop_document() {
        let xml = PropFind::AllProp { include: None }.into_xml().unwrap();
        let xml = std::str::from_utf8(&xml).unwrap();
        assert!(xml.contains("<D:propfind xmlns:D=\"DAV:\">"), "{xml}");
        assert!(xml.contains("<D:allprop/>"), "{xml}");
    }

    #[test]
    fn round_trips_all_variants() {
        let mut prop = Prop::default();
        prop.insert::<crate::properties::DisplayName>(Value::Empty);
        for propfind in [
            PropFind::PropName,
            PropFind::AllProp { include: None },
            PropFind::Prop(prop.clone()),
        ] {
            let xml = propfind.clone().into_xml().unwrap();
            assert_eq!(PropFind::from_xml(xml).unwrap(), propfind);
        }
    }
}
