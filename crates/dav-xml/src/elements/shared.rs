// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

unit_element!(
    /// The `shared` XML element
    /// ([RFC 4918 section 14.27](https://www.rfc-editor.org/rfc/rfc4918#section-14.27)).
    ///
    /// Marks a lock scope as shared.
    Shared,
    "shared"
);

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{Error, FromXml, IntoXml};

    #[test]
    fn parses_empty_element() {
        let parsed = Shared::from_xml(br#"<D:shared xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        assert_eq!(parsed, Shared);
    }

    #[test]
    fn rejects_text_content() {
        let error =
            Shared::from_xml(br#"<D:shared xmlns:D="DAV:">x</D:shared>"#.to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidValueType {
                element: "shared",
                ..
            }
        ));
    }

    #[test]
    fn writes_self_closing() {
        let xml = Shared.into_xml().unwrap();
        assert!(
            std::str::from_utf8(&xml)
                .unwrap()
                .ends_with(r#"<D:shared xmlns:D="DAV:"/>"#)
        );
    }
}
