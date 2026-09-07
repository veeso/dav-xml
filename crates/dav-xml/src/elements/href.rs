// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
        value.parse().map_err(Error::invalid::<Self>)
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
    type Err = http::uri::InvalidUri;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match http::Uri::from_str(value) {
            Ok(uri) => Ok(Self::Uri(uri)),
            Err(error) => {
                if is_valid_opaque_uri(value) {
                    Ok(Self::Raw(value.into()))
                } else {
                    Err(error)
                }
            }
        }
    }
}

fn is_valid_opaque_uri(value: &str) -> bool {
    let Some((scheme, remainder)) = value.split_once(':') else {
        return false;
    };
    if !is_valid_scheme(scheme) {
        return false;
    }

    let path_end = if let Some(index) = remainder.find(['?', '#']) {
        index
    } else {
        remainder.len()
    };
    let path = &remainder[..path_end];
    if path.is_empty() || path.starts_with('/') || !is_valid_uri_component(path) {
        return false;
    }

    let suffix = &remainder[path_end..];
    match suffix.strip_prefix('?') {
        Some(query_and_fragment) => match query_and_fragment.split_once('#') {
            Some((query, fragment)) => {
                is_valid_uri_component(query) && is_valid_uri_component(fragment)
            }
            None => is_valid_uri_component(query_and_fragment),
        },
        None => match suffix.strip_prefix('#') {
            Some(fragment) => is_valid_uri_component(fragment),
            None => true,
        },
    }
}

fn is_valid_scheme(value: &str) -> bool {
    let mut bytes = value.bytes();
    match bytes.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn is_valid_uri_component(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let Some(percent_encoded) = bytes.get(index + 1..index + 3) else {
                    return false;
                };
                if !percent_encoded.iter().all(u8::is_ascii_hexdigit) {
                    return false;
                }
                index += 3;
            }
            byte if is_uri_character(byte) => index += 1,
            _ => return false,
        }
    }
    true
}

fn is_uri_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'.'
                | b'_'
                | b'~'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b':'
                | b'@'
                | b'/'
                | b'?'
        )
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
    fn rejects_spaces_and_malformed_percent_escapes() {
        for text in [
            "https://example.org/a b",
            "urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4%2",
            "urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4%zz",
        ] {
            Href::from_str(text).unwrap_err();
        }
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
