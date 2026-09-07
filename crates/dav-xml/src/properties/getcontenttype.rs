// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use mime::Mime;

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `getcontenttype` property as defined in
/// [RFC 4918](http://webdav.org/specs/rfc4918.html#PROPERTY_getcontenttype).
#[derive(Clone, Debug, PartialEq)]
pub struct ContentType(pub Mime);

impl Element for ContentType {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "getcontenttype";
}

impl TryFrom<&Value> for ContentType {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map(Self)
            .map_err(Error::invalid::<Self>)
    }
}

impl From<ContentType> for Value {
    fn from(ContentType(content_type): ContentType) -> Value {
        content_type.to_string().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_writes_mime() {
        let content_type =
            ContentType::try_from(&Value::Text("text/html; charset=utf-8".into())).unwrap();
        assert_eq!(
            content_type.0,
            "text/html; charset=utf-8".parse::<Mime>().unwrap()
        );
        assert_eq!(
            Value::from(content_type),
            Value::Text("text/html; charset=utf-8".into())
        );
    }

    #[test]
    fn rejects_invalid_mime() {
        ContentType::try_from(&Value::Text("not a mime".into())).unwrap_err();
    }
}
