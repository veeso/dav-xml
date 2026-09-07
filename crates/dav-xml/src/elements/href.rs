// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::convert::Infallible;
use std::str::FromStr;

use bytestring::ByteString;

use crate::value::Value;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error};

/// The `href` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_href).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Href {
    /// A URI supported by [`http::Uri`].
    Uri(http::Uri),
    /// An opaque URI reference that [`http::Uri`] cannot parse.
    Raw(ByteString),
}

impl Href {
    /// The URI scheme, if present.
    #[must_use]
    pub fn scheme_str(&self) -> Option<&str> {
        match self {
            Self::Uri(uri) => uri.scheme_str(),
            Self::Raw(value) => value
                .split_once(':')
                .map(|(scheme, _)| scheme)
                .filter(|scheme| !scheme.is_empty()),
        }
    }

    /// The URI host, if present.
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        match self {
            Self::Uri(uri) => uri.host(),
            Self::Raw(_) => None,
        }
    }

    /// The URI path.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Self::Uri(uri) => uri.path(),
            Self::Raw(_) => "",
        }
    }
}

impl std::fmt::Display for Href {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uri(uri) => uri.fmt(f),
            Self::Raw(value) => value.fmt(f),
        }
    }
}

impl Element for Href {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "href";
}

impl TryFrom<&Value> for Href {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let value = value.as_str_of::<Self>()?;
        match value.parse() {
            Ok(href) => Ok(href),
            Err(never) => match never {},
        }
    }
}

impl From<Href> for Value {
    fn from(href: Href) -> Value {
        Value::Text(href.to_string().into())
    }
}

impl From<http::Uri> for Href {
    fn from(uri: http::Uri) -> Self {
        Self::Uri(uri)
    }
}

impl FromStr for Href {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(http::Uri::from_str(s).map_or_else(|_| Self::Raw(s.into()), Self::Uri))
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
        assert_eq!(absolute.path(), "/a%20b");
        let relative =
            Href::from_xml(br#"<D:href xmlns:D="DAV:">/a/b/</D:href>"#.to_vec()).unwrap();
        assert_eq!(relative.path(), "/a/b/");
    }

    #[test]
    fn converts_http_uri() {
        let href = Href::from("https://example.org/a".parse::<http::Uri>().unwrap());
        assert_eq!(href.host(), Some("example.org"));
    }

    #[test]
    fn http_uri_rejects_opaque_urn() {
        "urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4"
            .parse::<http::Uri>()
            .unwrap_err();
    }

    #[test]
    fn preserves_opaque_urn_as_raw_text() {
        let text = "urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4";
        let href = Href::from_xml(format!("<D:href xmlns:D=\"DAV:\">{text}</D:href>")).unwrap();
        assert_eq!(href, Href::Raw(text.into()));
        assert_eq!(href.scheme_str(), Some("urn"));
        assert_eq!(href.host(), None);
        assert_eq!(href.path(), "");
        assert_eq!(href.to_string(), text);
        assert_eq!(
            Href::from_xml(href.clone().into_xml().unwrap()).unwrap(),
            href
        );
    }

    #[test]
    fn represents_http_uri_as_uri_variant() {
        let href = Href::from("https://example.org/a".parse::<http::Uri>().unwrap());
        assert!(matches!(href, Href::Uri(_)));
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
        let xml = "/a".parse::<Href>().unwrap().into_xml().unwrap();
        assert!(
            std::str::from_utf8(&xml)
                .unwrap()
                .contains("<D:href xmlns:D=\"DAV:\">/a</D:href>")
        );
    }
}
