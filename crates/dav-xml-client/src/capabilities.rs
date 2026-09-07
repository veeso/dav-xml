// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Server capabilities advertised through the `DAV` and `Allow` headers.

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::response::capabilities;

    #[test]
    fn parses_dav_and_allow_headers() {
        let response = http::Response::builder()
            .status(200)
            .header("dav", "1, 2")
            .header("allow", "OPTIONS, GET, PROPFIND, LOCK")
            .body(Vec::new())
            .unwrap();
        let caps = capabilities(response).unwrap();
        assert!(caps.supports_locking());
        assert!(caps.allows(&http::Method::from_bytes(b"PROPFIND").unwrap()));
        assert!(!caps.allows(&http::Method::DELETE));
        assert_eq!(caps.allow.len(), 4);
    }

    #[test]
    fn missing_headers_yield_empty_capabilities() {
        let response = http::Response::builder()
            .status(200)
            .body(Vec::new())
            .unwrap();
        let caps = capabilities(response).unwrap();
        assert!(caps.dav.classes.is_empty());
        assert!(caps.allow.is_empty());
    }
}

use crate::headers::DavHeader;

/// The compliance classes and methods a server advertises through the `DAV`
/// and `Allow` response headers ([RFC 4918 section 10.1](https://www.rfc-editor.org/rfc/rfc4918#section-10.1)).
///
/// # Examples
///
/// ```
/// use dav_xml_client::Capabilities;
/// use dav_xml_client::headers::DavHeader;
///
/// let caps = Capabilities {
///     dav: "1, 2".parse::<DavHeader>().unwrap(),
///     allow: vec![http::Method::GET, http::Method::from_bytes(b"PROPFIND").unwrap()],
/// };
/// assert!(caps.supports_locking());
/// assert!(caps.allows(&http::Method::GET));
/// assert!(!caps.allows(&http::Method::DELETE));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// The parsed `DAV` header, listing the compliance classes the server
    /// advertises.
    pub dav: DavHeader,
    /// The methods listed in the `Allow` header.
    pub allow: Vec<http::Method>,
}

impl Capabilities {
    /// Whether the server advertises class `2` (locking) support.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::Capabilities;
    /// use dav_xml_client::headers::DavHeader;
    ///
    /// let caps = Capabilities {
    ///     dav: "1".parse::<DavHeader>().unwrap(),
    ///     allow: Vec::new(),
    /// };
    /// assert!(!caps.supports_locking());
    /// ```
    #[must_use]
    pub fn supports_locking(&self) -> bool {
        self.dav.supports_locking()
    }

    /// Whether `method` appears in the `Allow` header.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::Capabilities;
    /// use dav_xml_client::headers::DavHeader;
    ///
    /// let caps = Capabilities {
    ///     dav: DavHeader::default(),
    ///     allow: vec![http::Method::GET],
    /// };
    /// assert!(caps.allows(&http::Method::GET));
    /// assert!(!caps.allows(&http::Method::POST));
    /// ```
    #[must_use]
    pub fn allows(&self, method: &http::Method) -> bool {
        self.allow.iter().any(|allowed| allowed == method)
    }
}
