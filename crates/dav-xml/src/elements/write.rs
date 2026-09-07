// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

unit_element!(
    /// The `write` XML element
    /// ([RFC 4918 section 14.30](https://www.rfc-editor.org/rfc/rfc4918#section-14.30)).
    ///
    /// Identifies the write lock type.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::FromXml;
    /// use dav_xml::elements::Write;
    ///
    /// let write = Write::from_xml(br#"<D:write xmlns:D="DAV:"/>"#.to_vec()).unwrap();
    /// assert_eq!(write, Write);
    /// ```
    Write,
    "write"
);

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{Error, FromXml, IntoXml};

    #[test]
    fn parses_empty_element() {
        let parsed = Write::from_xml(br#"<D:write xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        assert_eq!(parsed, Write);
    }

    #[test]
    fn rejects_text_content() {
        let error =
            Write::from_xml(br#"<D:write xmlns:D="DAV:">x</D:write>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidValueType {
                element: "write",
                ..
            }
        ));
    }

    #[test]
    fn writes_self_closing() {
        let xml = Write.into_xml().unwrap();
        assert!(
            std::str::from_utf8(&xml)
                .unwrap()
                .ends_with(r#"<D:write xmlns:D="DAV:"/>"#)
        );
    }
}
