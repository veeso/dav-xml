// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A flattened view of `multistatus` response entries.

#[cfg(test)]
mod tests {
    use dav_xml::FromXml;
    use pretty_assertions::assert_eq;

    use super::*;

    const LISTING: &str = r#"<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dir/</D:href>
    <D:propstat>
      <D:prop><D:resourcetype><D:collection/></D:resourcetype><D:displayname>dir</D:displayname></D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
    <D:propstat>
      <D:prop><D:getcontentlength/></D:prop>
      <D:status>HTTP/1.1 404 Not Found</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/dir/file.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype/>
        <D:getcontentlength>12</D:getcontentlength>
        <D:getcontenttype>text/plain</D:getcontenttype>
        <D:getetag>"e1"</D:getetag>
        <D:getlastmodified>Mon, 01 Jan 2024 00:00:00 GMT</D:getlastmodified>
        <D:creationdate>2024-01-01T00:00:00Z</D:creationdate>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/gone</D:href>
    <D:status>HTTP/1.1 404 Not Found</D:status>
  </D:response>
</D:multistatus>"#;

    #[test]
    fn flattens_successful_propstats_only() {
        let ms = Multistatus::from_xml(LISTING.as_bytes().to_vec()).unwrap();
        let resources = Resource::from_multistatus(&ms);
        assert_eq!(resources.len(), 2, "status-only responses are skipped");
        let dir = &resources[0];
        assert!(dir.is_collection);
        assert_eq!(dir.display_name.as_deref(), Some("dir"));
        assert_eq!(dir.content_length, None);
        assert_eq!(dir.path(), "/dir/");
        assert_eq!(dir.name(), "dir");
        let file = &resources[1];
        assert!(!file.is_collection);
        assert_eq!(file.content_length, Some(12));
        assert_eq!(file.content_type, Some(mime::TEXT_PLAIN));
        assert_eq!(file.etag.as_ref().unwrap().tag, "e1");
        assert!(file.last_modified.is_some());
        assert!(file.creation_date.is_some());
        assert_eq!(file.name(), "file.txt");
    }

    #[test]
    fn invalid_property_values_become_none() {
        let xml = r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/f</D:href><D:propstat><D:prop><D:getcontentlength>abc</D:getcontentlength></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
        let ms = Multistatus::from_xml(xml.as_bytes().to_vec()).unwrap();
        let resources = Resource::from_multistatus(&ms);
        assert_eq!(resources[0].content_length, None);
    }

    #[test]
    fn trailing_slash_marks_collection_when_resourcetype_missing() {
        let xml = r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/d/</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
        let ms = Multistatus::from_xml(xml.as_bytes().to_vec()).unwrap();
        assert!(Resource::from_multistatus(&ms)[0].is_collection);
    }

    #[test]
    fn percent_decode_replaces_escapes() {
        assert_eq!(percent_decode("%20"), " ");
        assert_eq!(percent_decode("file.txt"), "file.txt");
    }

    #[test]
    fn percent_decode_keeps_invalid_escapes_literal() {
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn last_segment_percent_decodes_the_name() {
        assert_eq!(last_segment("/a/b%20c/"), "b c");
        assert_eq!(last_segment("/"), "");
    }
}

use dav_xml::elements::{Href, Multistatus, Prop, Response};
use dav_xml::properties::ETag;
use dav_xml::{Error as XmlError, Value};

/// One resource entry flattened out of a `multistatus` response.
///
/// Every successful `propstat` block of a [`Response::Propstat`] entry is
/// merged into a single [`Prop`], and a handful of well-known properties are
/// read out of it for convenience. A resource whose property could not be
/// parsed reads as [`None`] for that field rather than failing the whole
/// entry.
///
/// # Examples
///
/// ```
/// use dav_xml::FromXml;
/// use dav_xml::elements::Multistatus;
/// use dav_xml_client::Resource;
///
/// let xml = r#"<D:multistatus xmlns:D="DAV:">
///   <D:response>
///     <D:href>/notes/</D:href>
///     <D:propstat>
///       <D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop>
///       <D:status>HTTP/1.1 200 OK</D:status>
///     </D:propstat>
///   </D:response>
/// </D:multistatus>"#;
/// let multistatus = Multistatus::from_xml(xml.as_bytes().to_vec()).unwrap();
/// let resources = Resource::from_multistatus(&multistatus);
/// assert!(resources[0].is_collection);
/// assert_eq!(resources[0].name(), "notes");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Resource {
    /// The resource URI, as returned in `href`.
    pub href: Href,
    /// Whether `resourcetype` names a collection, or `href` ends in `/`.
    pub is_collection: bool,
    /// The `displayname` property, when present and valid.
    pub display_name: Option<String>,
    /// The `getcontentlength` property, when present and valid.
    pub content_length: Option<u64>,
    /// The `getcontenttype` property, when present and valid.
    pub content_type: Option<mime::Mime>,
    /// The `getetag` property, when present and valid.
    pub etag: Option<ETag>,
    /// The `getlastmodified` property, when present and valid.
    pub last_modified: Option<httpdate::HttpDate>,
    /// The `creationdate` property, when present and valid.
    pub creation_date: Option<time::OffsetDateTime>,
    /// Every property returned for this resource, merged from every
    /// successful `propstat` block.
    pub prop: Prop,
    name: String,
}

impl Resource {
    /// Flatten one `multistatus` response entry.
    ///
    /// Returns [`None`] for the [`Response::Status`] variant, which carries
    /// no properties, and for any successful `propstat` this merges the
    /// `prop` of every `propstat` whose status is a success into
    /// [`Resource::prop`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::FromXml;
    /// use dav_xml::elements::Response;
    /// use dav_xml_client::Resource;
    ///
    /// let xml = r#"<D:response xmlns:D="DAV:">
    ///   <D:href>/notes/a.txt</D:href>
    ///   <D:propstat>
    ///     <D:prop><D:getcontentlength>3</D:getcontentlength></D:prop>
    ///     <D:status>HTTP/1.1 200 OK</D:status>
    ///   </D:propstat>
    /// </D:response>"#;
    /// let response = Response::from_xml(xml.as_bytes().to_vec()).unwrap();
    /// let resource = Resource::from_response(&response).unwrap();
    /// assert_eq!(resource.content_length, Some(3));
    /// ```
    #[must_use]
    pub fn from_response(response: &Response) -> Option<Self> {
        let Response::Propstat { href, propstat, .. } = response else {
            return None;
        };

        let mut builder = Prop::builder();
        for entry in propstat.iter().filter(|entry| entry.status.is_success()) {
            if let Value::Map(map) = Value::from(entry.prop.clone()) {
                for (raw_name, raw_value) in map.iter_ordered() {
                    builder = builder.raw(raw_name.clone(), raw_value.clone());
                }
            }
        }
        let prop = builder.build();

        let is_collection = value(prop.resourcetype())
            .is_some_and(|resource_type| resource_type.is_collection())
            || href.path().ends_with('/');

        Some(Self {
            href: href.clone(),
            is_collection,
            display_name: value(prop.displayname()).map(|display_name| display_name.0.into()),
            content_length: value(prop.getcontentlength()).map(|content_length| content_length.0),
            content_type: value(prop.getcontenttype()).map(|content_type| content_type.0),
            etag: value(prop.getetag()),
            last_modified: value(prop.getlastmodified()).map(|last_modified| last_modified.0),
            creation_date: value(prop.creationdate()).map(|creation_date| creation_date.0),
            name: last_segment(href.path()),
            prop,
        })
    }

    /// Flatten every [`Response::Propstat`] entry of `multistatus`.
    ///
    /// Entries using the [`Response::Status`] variant, which carry no
    /// properties, are skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::FromXml;
    /// use dav_xml::elements::Multistatus;
    /// use dav_xml_client::Resource;
    ///
    /// let xml = r#"<D:multistatus xmlns:D="DAV:">
    ///   <D:response><D:href>/a</D:href><D:status>HTTP/1.1 404 Not Found</D:status></D:response>
    ///   <D:response><D:href>/b</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
    /// </D:multistatus>"#;
    /// let multistatus = Multistatus::from_xml(xml.as_bytes().to_vec()).unwrap();
    /// assert_eq!(Resource::from_multistatus(&multistatus).len(), 1);
    /// ```
    #[must_use]
    pub fn from_multistatus(multistatus: &Multistatus) -> Vec<Self> {
        multistatus
            .response
            .iter()
            .filter_map(Self::from_response)
            .collect()
    }

    /// The resource's URI path.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::FromXml;
    /// use dav_xml::elements::Multistatus;
    /// use dav_xml_client::Resource;
    ///
    /// let xml = r#"<D:multistatus xmlns:D="DAV:">
    ///   <D:response><D:href>/notes/a.txt</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
    /// </D:multistatus>"#;
    /// let multistatus = Multistatus::from_xml(xml.as_bytes().to_vec()).unwrap();
    /// assert_eq!(Resource::from_multistatus(&multistatus)[0].path(), "/notes/a.txt");
    /// ```
    #[must_use]
    pub fn path(&self) -> &str {
        self.href.path()
    }

    /// The last non-empty, percent-decoded path segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::FromXml;
    /// use dav_xml::elements::Multistatus;
    /// use dav_xml_client::Resource;
    ///
    /// let xml = r#"<D:multistatus xmlns:D="DAV:">
    ///   <D:response><D:href>/notes/a%20b.txt</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
    /// </D:multistatus>"#;
    /// let multistatus = Multistatus::from_xml(xml.as_bytes().to_vec()).unwrap();
    /// assert_eq!(Resource::from_multistatus(&multistatus)[0].name(), "a b.txt");
    /// ```
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Read a typed [`Prop`] accessor result, discarding a missing, empty or
/// invalid property.
#[expect(
    clippy::option_option,
    reason = "mirrors the Option<Option<Result<_, _>>> shape returned by Prop's typed accessors"
)]
fn value<T>(read: Option<Option<Result<T, XmlError>>>) -> Option<T> {
    match read {
        Some(Some(Ok(value))) => Some(value),
        _ => None,
    }
}

/// The last non-empty segment of `path`, percent-decoded.
fn last_segment(path: &str) -> String {
    let segment = path
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or_default();
    percent_decode(segment)
}

/// A small percent-decoder for URI path segments.
///
/// A `%` not followed by two hexadecimal digits, or a decoded sequence that
/// is not valid UTF-8, is left untouched rather than rejected.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let byte = bytes
                .get(index + 1..index + 3)
                .and_then(|hex| std::str::from_utf8(hex).ok())
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            if let Some(byte) = byte {
                decoded.push(byte);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_error| input.to_owned())
}
