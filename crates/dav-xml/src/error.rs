// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bytes::Bytes;

use crate::Element;

/// Result alias used across the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised while reading or writing `WebDAV` XML.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The element holds a different kind of content than expected.
    #[error("`{element}` element: expected {expected}")]
    InvalidValueType {
        /// Local name of the element being parsed.
        element: &'static str,
        /// Human-readable description of the expected content.
        expected: &'static str,
    },
    /// A mandatory child element is absent.
    #[error("missing `{element}` element inside `{parent}`")]
    MissingElement {
        /// Local name of the parent element.
        parent: &'static str,
        /// Local name of the missing child.
        element: &'static str,
    },
    /// Mutually exclusive children appear together.
    #[error("conflicting elements inside `{parent}`: {elements}")]
    ConflictingElements {
        /// Local name of the parent element.
        parent: &'static str,
        /// Description of the conflicting children.
        elements: &'static str,
    },
    /// The text of an element could not be converted into its typed value.
    #[error("invalid `{element}` element: {source}")]
    InvalidElement {
        /// Local name of the element.
        element: &'static str,
        /// The conversion error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A prefix is bound to an empty namespace URI.
    #[error("invalid namespace declaration: {0:?}")]
    InvalidNamespace(Bytes),
    /// A closing tag does not match its opening tag or content is invalid.
    #[error("unexpected tag at byte {position}")]
    UnexpectedTag {
        /// Byte offset of the offending event.
        position: u64,
    },
    /// An entity reference that is neither predefined nor a character
    /// reference.
    #[error("unknown entity reference: `&{0};`")]
    UnknownEntity(String),
    /// The underlying XML parser or writer failed.
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
    /// Writing to the output failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The document is not valid UTF-8.
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

impl Error {
    /// Wrap a conversion error with the name of the element being parsed.
    pub fn invalid<E: Element>(
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::InvalidElement {
            element: E::LOCAL_NAME,
            source: source.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::elements::Href;

    #[test]
    fn invalid_element_names_the_element() {
        let error = Error::invalid::<Href>(std::io::Error::other("boom"));
        assert_eq!(error.to_string(), "invalid `href` element: boom");
    }

    #[test]
    fn missing_element_names_parent_and_child() {
        let error = Error::MissingElement {
            parent: "propstat",
            element: "status",
        };
        assert_eq!(
            error.to_string(),
            "missing `status` element inside `propstat`"
        );
    }

    #[test]
    fn invalid_value_type_names_expectation() {
        let error = Error::InvalidValueType {
            element: "href",
            expected: "text",
        };
        assert_eq!(error.to_string(), "`href` element: expected text");
    }
}
