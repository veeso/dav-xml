// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

href_element!(
    /// The `lockroot` XML element
    /// ([RFC 4918 section 14.12](https://www.rfc-editor.org/rfc/rfc4918#section-14.12)).
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::{Href, LockRoot};
    /// use dav_xml::IntoXml;
    ///
    /// let root = LockRoot::from("https://example.org/locked".parse::<Href>().unwrap());
    /// let xml = root.into_xml().unwrap();
    /// assert!(std::str::from_utf8(&xml).unwrap().contains("<D:lockroot"));
    /// ```
    LockRoot,
    "lockroot"
);

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{Error, FromXml, IntoXml};

    const XML: &[u8] = br#"<D:lockroot xmlns:D="DAV:"><D:href>urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4</D:href></D:lockroot>"#;

    #[test]
    fn parses_href() {
        let root = LockRoot::from_xml(XML.to_vec()).unwrap();
        assert_eq!(root.0.scheme_str(), Some("urn"));
    }

    #[test]
    fn missing_href_is_reported() {
        let error = LockRoot::from_xml(br#"<D:lockroot xmlns:D="DAV:"/>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "lockroot",
                element: "href"
            }
        ));
    }

    #[test]
    fn round_trips() {
        let root = LockRoot::from_xml(XML.to_vec()).unwrap();
        assert_eq!(
            LockRoot::from_xml(root.clone().into_xml().unwrap()).unwrap(),
            root
        );
    }
}
