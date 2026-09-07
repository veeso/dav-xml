// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `getcontentlength` property as defined in
/// [RFC 4918](http://webdav.org/specs/rfc4918.html#PROPERTY_getcontentlength).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContentLength(pub u64);

impl Element for ContentLength {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "getcontentlength";
}

impl TryFrom<&Value> for ContentLength {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map(Self)
            .map_err(Error::invalid::<Self>)
    }
}

impl From<ContentLength> for Value {
    fn from(ContentLength(len): ContentLength) -> Value {
        len.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_non_negative_integer() {
        assert_eq!(
            ContentLength::try_from(&Value::Text("4525".into()))
                .unwrap()
                .0,
            4525
        );
    }

    #[test]
    fn rejects_invalid_integer() {
        ContentLength::try_from(&Value::Text("-1".into())).unwrap_err();
        ContentLength::try_from(&Value::Text("abc".into())).unwrap_err();
    }
}
