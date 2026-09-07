// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `creationdate` property as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#PROPERTY_creationdate).
#[derive(Clone, Debug, PartialEq)]
pub struct CreationDate(pub OffsetDateTime);

impl Element for CreationDate {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "creationdate";
}

impl TryFrom<&Value> for CreationDate {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        OffsetDateTime::parse(value.as_str_of::<Self>()?, &Rfc3339)
            .map(Self)
            .map_err(Error::invalid::<Self>)
    }
}

impl From<CreationDate> for Value {
    fn from(CreationDate(datetime): CreationDate) -> Value {
        datetime.format(&Rfc3339).map_or(Value::Empty, Value::from)
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn parses_rfc3339() {
        let date =
            CreationDate::try_from(&Value::Text("1997-12-01T17:42:21-08:00".into())).unwrap();
        assert_eq!(date.0, datetime!(1997-12-01 17:42:21 -08:00));
    }

    #[test]
    fn rejects_http_date() {
        let error = CreationDate::try_from(&Value::Text("Mon, 12 Jan 1998 09:25:56 GMT".into()))
            .unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidElement {
                element: "creationdate",
                ..
            }
        ));
    }

    #[test]
    fn writes_rfc3339() {
        let value: Value = CreationDate(datetime!(2020-01-02 03:04:05 UTC)).into();
        assert_eq!(value, Value::Text("2020-01-02T03:04:05Z".into()));
    }
}
