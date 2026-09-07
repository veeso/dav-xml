// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

unit_element!(
    /// The `collection` XML element
    /// ([RFC 4918 section 14.3](https://www.rfc-editor.org/rfc/rfc4918#section-14.3)).
    ///
    /// Identifies a resource as a collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::FromXml;
    /// use dav_xml::elements::Collection;
    ///
    /// let collection =
    ///     Collection::from_xml(br#"<D:collection xmlns:D="DAV:"/>"#.to_vec()).unwrap();
    /// assert_eq!(collection, Collection);
    /// ```
    Collection,
    "collection"
);

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let collection =
            Collection::from_xml(br#"<D:collection xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        let output = collection.into_xml().unwrap();
        assert_eq!(Collection::from_xml(output).unwrap(), Collection);
    }
}
