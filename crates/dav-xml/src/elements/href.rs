// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::str::FromStr;

use crate::value::Value;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error};

/// The `href` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_href).
#[derive(Clone, Debug, PartialEq)]
pub struct Href(pub http::Uri);

impl Element for Href {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "href";
}

impl TryFrom<&Value> for Href {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map(Self)
            .map_err(Error::invalid::<Self>)
    }
}

impl From<Href> for Value {
    fn from(Href(uri): Href) -> Value {
        Value::Text(uri.to_string().into())
    }
}

impl From<http::Uri> for Href {
    fn from(uri: http::Uri) -> Self {
        Href(uri)
    }
}

impl FromStr for Href {
    type Err = <http::Uri as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        http::Uri::from_str(s).map(Href)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_absolute_and_relative() {
        let absolute =
            Href::from_xml(br#"<D:href xmlns:D="DAV:">http://x/a%20b</D:href>"#.to_vec()).unwrap();
        assert_eq!(absolute.0.path(), "/a%20b");
        let relative =
            Href::from_xml(br#"<D:href xmlns:D="DAV:">/a/b/</D:href>"#.to_vec()).unwrap();
        assert_eq!(relative.0.path(), "/a/b/");
    }

    #[test]
    fn rejects_non_text() {
        let error =
            Href::from_xml(br#"<D:href xmlns:D="DAV:"><x/></D:href>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidValueType {
                element: "href",
                ..
            }
        ));
    }

    #[test]
    fn writes_text() {
        let xml = Href("/a".parse().unwrap()).into_xml().unwrap();
        assert!(
            std::str::from_utf8(&xml)
                .unwrap()
                .contains("<D:href xmlns:D=\"DAV:\">/a</D:href>")
        );
    }
}
