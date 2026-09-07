// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `propname` XML element as defined in RFC 4918 section 14.21.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropName;

impl Element for PropName {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "propname";
}

impl TryFrom<&Value> for PropName {
    type Error = Error;

    fn try_from(_: &Value) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl From<PropName> for Value {
    fn from(_: PropName) -> Value {
        Value::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let propname = PropName::from_xml(br#"<D:propname xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        let output = propname.into_xml().unwrap();
        assert_eq!(PropName::from_xml(output).unwrap(), PropName);
    }
}
