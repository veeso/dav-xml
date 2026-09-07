// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

unit_element!(
    /// The `propname` XML element
    /// ([RFC 4918 section 14.21](https://www.rfc-editor.org/rfc/rfc4918#section-14.21)).
    ///
    /// Requests the names of the properties of a resource.
    PropName,
    "propname"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_writes_and_round_trips() {
        let propname = PropName::from_xml(br#"<D:propname xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        let output = propname.into_xml().unwrap();
        assert_eq!(PropName::from_xml(output).unwrap(), PropName);
    }
}
