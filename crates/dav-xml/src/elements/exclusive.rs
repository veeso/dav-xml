// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

unit_element!(
    /// The `exclusive` XML element
    /// ([RFC 4918 section 14.6](https://www.rfc-editor.org/rfc/rfc4918#section-14.6)).
    ///
    /// Marks a lock scope as exclusive.
    Exclusive,
    "exclusive"
);

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{Error, FromXml, IntoXml};

    #[test]
    fn parses_empty_element() {
        let parsed = Exclusive::from_xml(br#"<D:exclusive xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        assert_eq!(parsed, Exclusive);
    }

    #[test]
    fn rejects_text_content() {
        let error = Exclusive::from_xml(br#"<D:exclusive xmlns:D="DAV:">x</D:exclusive>"#.to_vec())
            .unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidValueType {
                element: "exclusive",
                ..
            }
        ));
    }

    #[test]
    fn writes_self_closing() {
        let xml = Exclusive.into_xml().unwrap();
        assert!(
            std::str::from_utf8(&xml)
                .unwrap()
                .ends_with(r#"<D:exclusive xmlns:D="DAV:"/>"#)
        );
    }
}
