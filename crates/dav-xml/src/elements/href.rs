// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::str::FromStr;

use iri_string::types::UriReferenceString;

use crate::value::Value;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error};

/// The `href` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_href).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Href(pub HrefUri);

/// A URI reference used in an [`Href`].
///
/// Unlike [`http::Uri`], this supports opaque URI references such as `WebDAV`
/// lock tokens using the `urn:` scheme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HrefUri(UriReferenceString);

impl HrefUri {
    /// The URI scheme, if the reference has one.
    #[must_use]
    pub fn scheme_str(&self) -> Option<&str> {
        self.0.scheme_str()
    }

    /// The URI host, if the reference has an authority component.
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        self.0
            .authority_components()
            .map(|authority| authority.host())
    }

    /// The URI path.
    #[must_use]
    pub fn path(&self) -> &str {
        self.0.path_str()
    }
}

impl std::fmt::Display for HrefUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for HrefUri {
    type Err = iri_string::validate::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        UriReferenceString::try_from(value).map(Self)
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

impl From<http::Uri> for HrefUri {
    fn from(uri: http::Uri) -> Self {
        Self::from_str(&uri.to_string()).expect("an HTTP URI is a valid URI reference")
    }
}

impl From<http::Uri> for Href {
    fn from(uri: http::Uri) -> Self {
        Self(uri.into())
    }
}

impl FromStr for Href {
    type Err = iri_string::validate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HrefUri::from_str(s).map(Href)
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
    fn converts_http_uri() {
        let href = Href::from("https://example.org/a".parse::<http::Uri>().unwrap());
        assert_eq!(href.0.host(), Some("example.org"));
    }

    #[test]
    fn http_uri_rejects_opaque_urn() {
        "urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4"
            .parse::<http::Uri>()
            .unwrap_err();
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
