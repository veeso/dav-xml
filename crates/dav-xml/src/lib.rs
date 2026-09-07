// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]
#![doc = "The RFC 4918 coverage table above documents the supported XML elements, live properties, and pre- and postcondition codes."]

mod element;
#[macro_use]
mod macros;
pub mod elements;
mod error;
pub mod properties;
mod read;
mod utils;
mod value;
mod write;

use bytes::{BufMut, Bytes};

pub use self::element::{Element, ElementName};
pub use self::error::{Error, Result};
pub use self::value::{Value, ValueMap};

/// The `DAV:` namespace URI.
///
/// # Examples
///
/// ```
/// assert_eq!(dav_xml::DAV_NAMESPACE, "DAV:");
/// ```
pub const DAV_NAMESPACE: &str = "DAV:";

/// The prefix this crate uses when writing `DAV:` elements.
///
/// # Examples
///
/// ```
/// assert_eq!(dav_xml::DAV_PREFIX, "D");
/// ```
pub const DAV_PREFIX: &str = "D";

/// Deserialization from an XML document.
///
/// Implemented for every `E: Element + TryFrom<&Value, Error = Error>`; do not
/// implement it directly.
///
/// # Examples
///
/// ```
/// use dav_xml::FromXml;
/// use dav_xml::elements::Href;
///
/// let href = Href::from_xml(br#"<D:href xmlns:D="DAV:">/a</D:href>"#.to_vec()).unwrap();
/// assert_eq!(href.path(), "/a");
/// ```
pub trait FromXml: Sized {
    /// Parse `xml` into `Self`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the document is not well formed, not UTF-8, or
    /// does not contain the expected root element.
    fn from_xml(xml: impl Into<Bytes>) -> Result<Self>;
}

impl FromXml for Value {
    fn from_xml(xml: impl Into<Bytes>) -> Result<Self> {
        read::read_xml(xml)
    }
}

impl<E> FromXml for E
where
    E: Element + for<'v> TryFrom<&'v Value, Error = Error>,
{
    fn from_xml(xml: impl Into<Bytes>) -> Result<Self> {
        Value::from_xml(xml)?.as_map()?.get_required::<E, E>()
    }
}

/// Serialization to an XML document.
///
/// Implemented for every `T: Element + Into<Value>`; do not implement it
/// directly.
///
/// # Examples
///
/// ```
/// use dav_xml::IntoXml;
/// use dav_xml::elements::Href;
///
/// let xml = "/a".parse::<Href>().unwrap().into_xml().unwrap();
/// assert!(std::str::from_utf8(&xml)
///     .unwrap()
///     .contains("<D:href xmlns:D=\"DAV:\">/a</D:href>"));
/// ```
pub trait IntoXml: Sized {
    /// Write `self` as an XML document to `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when `writer` fails or [`Error::Xml`] when the
    /// document cannot be encoded.
    fn write_xml(self, writer: impl std::io::Write) -> Result<()>;

    /// Serialize `self` into an in-memory XML document.
    ///
    /// # Errors
    ///
    /// See [`IntoXml::write_xml`].
    fn into_xml(self) -> Result<Bytes> {
        let mut xml = bytes::BytesMut::new().writer();
        self.write_xml(&mut xml)?;
        Ok(xml.into_inner().freeze())
    }
}

impl<T> IntoXml for T
where
    T: Element + Into<Value>,
{
    fn write_xml(self, writer: impl std::io::Write) -> Result<()> {
        self.validate()?;
        write::write_xml::<T>(writer, self.into())
    }
}
