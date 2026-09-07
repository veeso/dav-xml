// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `allprop` XML element as defined in RFC 4918 section 14.2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllProp;

impl Element for AllProp {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "allprop";
}

impl TryFrom<&Value> for AllProp {
    type Error = Error;

    fn try_from(_: &Value) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl From<AllProp> for Value {
    fn from(_: AllProp) -> Value {
        Value::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let allprop = AllProp::from_xml(br#"<D:allprop xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        let output = allprop.into_xml().unwrap();
        assert_eq!(AllProp::from_xml(output).unwrap(), AllProp);
    }
}
