// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bytestring::ByteString;

use crate::element::ElementName;
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `include` XML element as defined in RFC 4918 section 14.8.
#[derive(Clone, Debug, PartialEq)]
pub struct Include(pub Vec<ElementName<ByteString>>);

impl Element for Include {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "include";
}

impl TryFrom<&Value> for Include {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        Ok(Self(map.as_ref().keys().cloned().collect()))
    }
}

impl From<Include> for Value {
    fn from(Include(names): Include) -> Value {
        let mut map = ValueMap::new();
        for name in names {
            map.insert_raw(name, Value::Empty);
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
        let include = Include::from_xml(
            br#"<D:include xmlns:D="DAV:"><D:displayname/></D:include>"#.to_vec(),
        )
        .unwrap();
        assert_eq!(include.0.len(), 1);
        let output = include.clone().into_xml().unwrap();
        assert_eq!(Include::from_xml(output).unwrap(), include);
    }
}
