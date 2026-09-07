// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use httpdate::HttpDate;

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `getlastmodified` property as defined in
/// [RFC 4918](http://webdav.org/specs/rfc4918.html#PROPERTY_getlastmodified).
#[derive(Clone, Debug, PartialEq)]
pub struct LastModified(pub HttpDate);

impl Element for LastModified {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "getlastmodified";
}

impl TryFrom<&Value> for LastModified {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map(Self)
            .map_err(Error::invalid::<Self>)
    }
}

impl From<LastModified> for Value {
    fn from(LastModified(date): LastModified) -> Value {
        date.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_writes_http_date() {
        let date = "Mon, 12 Jan 1998 09:25:56 GMT";
        let last_modified = LastModified::try_from(&Value::Text(date.into())).unwrap();
        assert_eq!(Value::from(last_modified), Value::Text(date.into()));
    }

    #[test]
    fn rejects_rfc3339() {
        LastModified::try_from(&Value::Text("1998-01-12T09:25:56Z".into())).unwrap_err();
    }
}
