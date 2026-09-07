// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

unit_element!(
    /// The `allprop` XML element
    /// ([RFC 4918 section 14.2](https://www.rfc-editor.org/rfc/rfc4918#section-14.2)).
    ///
    /// Requests all properties of a resource.
    AllProp,
    "allprop"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let allprop = AllProp::from_xml(br#"<D:allprop xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        let output = allprop.into_xml().unwrap();
        assert_eq!(AllProp::from_xml(output).unwrap(), AllProp);
    }
}
