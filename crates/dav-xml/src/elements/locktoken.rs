// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

href_element!(
    /// The `locktoken` XML element
    /// ([RFC 4918 section 14.14](https://www.rfc-editor.org/rfc/rfc4918#section-14.14)).
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::{Href, LockToken};
    /// use dav_xml::IntoXml;
    ///
    /// let token = LockToken::from(Href("https://example.org/token".parse().unwrap()));
    /// let xml = token.into_xml().unwrap();
    /// assert!(std::str::from_utf8(&xml).unwrap().contains("<D:locktoken"));
    /// ```
    LockToken,
    "locktoken"
);

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{Error, FromXml, IntoXml};

    const XML: &[u8] = br#"<D:locktoken xmlns:D="DAV:"><D:href>urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4</D:href></D:locktoken>"#;

    #[test]
    fn parses_href() {
        let token = LockToken::from_xml(XML.to_vec()).unwrap();
        assert_eq!(token.0.0.scheme_str(), Some("urn"));
    }

    #[test]
    fn missing_href_is_reported() {
        let error = LockToken::from_xml(br#"<D:locktoken xmlns:D="DAV:"/>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "locktoken",
                element: "href"
            }
        ));
    }

    #[test]
    fn round_trips() {
        let token = LockToken::from_xml(XML.to_vec()).unwrap();
        assert_eq!(
            LockToken::from_xml(token.clone().into_xml().unwrap()).unwrap(),
            token
        );
    }
}
